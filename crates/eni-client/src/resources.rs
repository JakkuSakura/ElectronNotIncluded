//! Cross-boundary resources used to drive the client from the REST/CLI layer.

use bevy::prelude::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};

#[derive(Resource, Clone, Copy, Debug)]
pub struct DebugMode;

pub struct FramebufferCaptureRequest {
    pub output: PathBuf,
    pub response: mpsc::Sender<Result<Vec<u8>, String>>,
}

#[derive(Resource, Clone)]
pub struct FramebufferCaptureQueue(pub Arc<Mutex<mpsc::Receiver<FramebufferCaptureRequest>>>);

#[derive(Resource, Clone)]
pub struct FramebufferTarget(pub Handle<Image>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadlessStartState {
    Menu,
    Playing,
}

#[derive(Resource, Clone, Copy)]
pub struct HeadlessMode {
    pub start_state: HeadlessStartState,
}

pub struct ClientControlRequest {
    pub action: ClientControlAction,
    pub response: mpsc::Sender<Result<(), String>>,
}

pub enum ClientControlAction {
    State(HeadlessStartState),
}

#[derive(Resource, Clone)]
pub struct ClientControlQueue(pub Arc<Mutex<mpsc::Receiver<ClientControlRequest>>>);

#[derive(Resource, Default)]
pub(crate) struct EguiFontsConfigured(pub(crate) bool);
