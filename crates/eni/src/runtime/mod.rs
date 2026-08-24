use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::{get, post},
};
use eni_domain::{ChunkCoord, DataError, GameClock, GameData, WorldChunk, generate_chunk};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    net::TcpListener,
    sync::Mutex,
    time::{Duration, interval},
};

use crate::cli::Operation;
use eni_client::{
    ClientControlAction, ClientControlRequest, FramebufferCaptureRequest, HeadlessStartState,
};

mod preview;

pub(crate) use preview::save_world_png;

/// Fixed default world-generation seed used when the caller does not
/// override it; kept as a constant (rather than random) so `verify`/
/// `headless` runs are reproducible by default.
pub const DEFAULT_SEED: u32 = 1;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to load game data: {0}")]
    Data(#[from] DataError),
    #[error("failed to access runtime file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to encode runtime image: {0}")]
    Image(#[from] image::ImageError),
    #[error("failed to serialize runtime response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid server response: {0}")]
    Http(#[from] axum::http::Error),
}

pub struct LoadedRuntime {
    pub game_data: GameData,
    pub seed: u32,
}

pub fn load_runtime(
    data_dir: impl AsRef<Path>,
    seed: Option<u32>,
) -> Result<LoadedRuntime, RuntimeError> {
    let game_data = GameData::load(data_dir)?;
    Ok(LoadedRuntime {
        game_data,
        seed: seed.unwrap_or(DEFAULT_SEED),
    })
}

pub fn verify(data_dir: impl AsRef<Path>, seed: Option<u32>) -> Result<WorldSummary, RuntimeError> {
    let runtime = load_runtime(data_dir, seed)?;
    let chunk = generate_chunk(
        runtime.seed,
        ChunkCoord { x: 0, y: 0 },
        &runtime.game_data.substances,
    );
    Ok(WorldSummary::from_state(
        runtime.seed,
        &runtime.game_data,
        &chunk,
    ))
}

pub fn run_headless(
    data_dir: impl AsRef<Path>,
    seed: Option<u32>,
    real_seconds: f64,
) -> Result<RuntimeSnapshot, RuntimeError> {
    let mut state = RuntimeState::new(load_runtime(data_dir, seed)?);
    state.advance(real_seconds);
    Ok(state.snapshot())
}

pub fn operate(
    data_dir: impl AsRef<Path>,
    seed: Option<u32>,
    operation: Operation,
    real_seconds: f64,
) -> Result<RuntimeSnapshot, RuntimeError> {
    let mut state = RuntimeState::new(load_runtime(data_dir, seed)?);
    match operation {
        Operation::AdvanceTime => state.advance(real_seconds),
        Operation::Pause => state.paused = true,
        Operation::Resume => state.paused = false,
    }
    Ok(state.snapshot())
}

#[derive(Clone)]
pub struct RuntimeState {
    pub game_data: Arc<GameData>,
    pub seed: u32,
    pub chunk: WorldChunk,
    pub clock: GameClock,
    pub paused: bool,
}

impl RuntimeState {
    fn new(runtime: LoadedRuntime) -> Self {
        let chunk = generate_chunk(
            runtime.seed,
            ChunkCoord { x: 0, y: 0 },
            &runtime.game_data.substances,
        );
        Self {
            game_data: Arc::new(runtime.game_data),
            seed: runtime.seed,
            chunk,
            clock: GameClock::default(),
            paused: false,
        }
    }

    fn advance(&mut self, real_seconds: f64) {
        if !self.paused {
            self.clock.advance_real_seconds(real_seconds);
            eni_server::tick_chunk(&mut self.chunk, &self.game_data);
        }
    }

    fn snapshot(&self) -> RuntimeSnapshot {
        let date_time = self.clock.date_time();
        RuntimeSnapshot {
            calendar: format!(
                "{} / {} {:02}:{:02}:{:02}",
                date_time.year, date_time.day, date_time.hour, date_time.minute, date_time.second
            ),
            paused: self.paused,
            world: WorldSummary::from_state(self.seed, &self.game_data, &self.chunk),
        }
    }
}

pub type SharedRuntime = Arc<Mutex<RuntimeState>>;

#[derive(Debug, Serialize)]
pub struct RuntimeSnapshot {
    pub calendar: String,
    pub paused: bool,
    pub world: WorldSummary,
}

#[derive(Debug, Serialize)]
pub struct WorldSummary {
    pub seed: u32,
    pub substance_count: usize,
    pub reaction_count: usize,
    pub total_mass_kg: f32,
}

impl WorldSummary {
    fn from_state(seed: u32, game_data: &GameData, chunk: &WorldChunk) -> Self {
        Self {
            seed,
            substance_count: game_data.substances.len(),
            reaction_count: game_data.reactions.len(),
            total_mass_kg: chunk.tiles.data.iter().map(|tile| tile.total_mass()).sum(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OperationRequest {
    pub operation: OperationKind,
    #[serde(default)]
    pub seconds: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    AdvanceTime,
    Pause,
    Resume,
}

pub fn spawn_server(
    data_dir: PathBuf,
    seed: Option<u32>,
    bind: std::net::SocketAddr,
    screenshot_sender: Option<mpsc::Sender<FramebufferCaptureRequest>>,
    control_sender: Option<mpsc::Sender<ClientControlRequest>>,
) -> Result<(), RuntimeError> {
    std::thread::Builder::new()
        .name("eni-rest-server".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::error!(%error, "failed to create REST server runtime");
                    return;
                }
            };
            if let Err(error) = runtime.block_on(serve(
                data_dir,
                seed,
                bind,
                screenshot_sender,
                control_sender,
            )) {
                tracing::error!(%error, "REST server stopped");
            }
        })
        .map(|_| ())
        .map_err(RuntimeError::Io)
}

pub async fn serve(
    data_dir: PathBuf,
    seed: Option<u32>,
    bind: std::net::SocketAddr,
    screenshot_sender: Option<mpsc::Sender<FramebufferCaptureRequest>>,
    control_sender: Option<mpsc::Sender<ClientControlRequest>>,
) -> Result<(), RuntimeError> {
    let runtime = RuntimeState::new(load_runtime(data_dir, seed)?);
    let shared = Arc::new(Mutex::new(runtime));
    let runtime_for_ticker = Arc::clone(&shared);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            runtime_for_ticker.lock().await.advance(1.0);
        }
    });
    let service_state = RuntimeServiceState {
        runtime: shared,
        screenshot_sender,
        control_sender,
    };
    let router = Router::new()
        .route("/health", get(health))
        .route("/api/state", get(state))
        .route("/api/world", get(world))
        .route("/api/render/world.png", get(render_world))
        .route("/api/render/framebuffer.png", get(render_framebuffer))
        .route("/api/control/state", post(control_state))
        .route("/api/operate", post(operate_request))
        .with_state(service_state);
    let listener = TcpListener::bind(bind).await?;
    tracing::info!(address = %bind, "runtime REST server listening");
    axum::serve(listener, router)
        .await
        .map_err(std::io::Error::other)?;
    Ok(())
}

#[derive(Clone)]
struct RuntimeServiceState {
    runtime: SharedRuntime,
    screenshot_sender: Option<mpsc::Sender<FramebufferCaptureRequest>>,
    control_sender: Option<mpsc::Sender<ClientControlRequest>>,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "eni" }))
}

async fn state(State(state): State<RuntimeServiceState>) -> Json<RuntimeSnapshot> {
    Json(state.runtime.lock().await.snapshot())
}

async fn world(State(state): State<RuntimeServiceState>) -> Json<WorldSummary> {
    let runtime = state.runtime.lock().await;
    Json(WorldSummary::from_state(
        runtime.seed,
        &runtime.game_data,
        &runtime.chunk,
    ))
}

async fn operate_request(
    State(state): State<RuntimeServiceState>,
    Json(request): Json<OperationRequest>,
) -> Json<RuntimeSnapshot> {
    let mut runtime = state.runtime.lock().await;
    match request.operation {
        OperationKind::AdvanceTime => runtime.advance(request.seconds),
        OperationKind::Pause => runtime.paused = true,
        OperationKind::Resume => runtime.paused = false,
    }
    Json(runtime.snapshot())
}

async fn render_world(
    State(state): State<RuntimeServiceState>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let runtime = state.runtime.lock().await;
    let bytes = preview::render_chunk_png(&runtime.chunk).map_err(internal_error)?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    Ok(response)
}

async fn render_framebuffer(
    State(state): State<RuntimeServiceState>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let Some(sender) = state.screenshot_sender.as_ref() else {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            "framebuffer capture is unavailable without a Bevy runtime".into(),
        ));
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| internal_error(RuntimeError::Io(std::io::Error::other(error))))?
        .as_nanos();
    let output = PathBuf::from(format!("target/preview/framebuffer-{timestamp}.png"));
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| internal_error(RuntimeError::Io(error)))?;
    }
    let (response_sender, response_receiver) = mpsc::channel();
    sender
        .send(FramebufferCaptureRequest {
            output,
            response: response_sender,
        })
        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, error.to_string()))?;
    let bytes = tokio::task::spawn_blocking(move || response_receiver.recv())
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, error.to_string()))?
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    Ok(response)
}

#[derive(Debug, Deserialize)]
struct ControlStateRequest {
    state: ControlState,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControlState {
    Menu,
    Playing,
}

async fn control_state(
    State(state): State<RuntimeServiceState>,
    Json(request): Json<ControlStateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Some(sender) = state.control_sender.as_ref() else {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            "client state control is unavailable without a Bevy runtime".into(),
        ));
    };
    let requested_state = request.state;
    let state_name = match requested_state {
        ControlState::Menu => "menu",
        ControlState::Playing => "playing",
    };
    let (response_sender, response_receiver) = mpsc::channel();
    sender
        .send(ClientControlRequest {
            action: ClientControlAction::State(match requested_state {
                ControlState::Menu => HeadlessStartState::Menu,
                ControlState::Playing => HeadlessStartState::Playing,
            }),
            response: response_sender,
        })
        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, error.to_string()))?;
    tokio::task::spawn_blocking(move || response_receiver.recv())
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, error.to_string()))?
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    Ok(Json(serde_json::json!({
        "state": state_name,
        "status": "accepted"
    })))
}

fn internal_error(error: RuntimeError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
