use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Instant;

use crate::engine::{perform_cycle, MotionConfig};
use crate::platform::{NativeBackend, PlatformBackend};

pub enum WorkerEvent {
    BackendReady(String),
    RunningChanged(bool),
    Info(String),
    Error(String),
}

enum WorkerCommand {
    Start(MotionConfig),
    Update(MotionConfig),
    Stop,
    Shutdown,
}

pub struct WorkerHandle {
    command_tx: Sender<WorkerCommand>,
    event_rx: Receiver<WorkerEvent>,
}

impl WorkerHandle {
    pub fn start(&self, config: MotionConfig) -> Result<(), String> {
        self.command_tx
            .send(WorkerCommand::Start(config))
            .map_err(|err| err.to_string())
    }

    pub fn update(&self, config: MotionConfig) -> Result<(), String> {
        self.command_tx
            .send(WorkerCommand::Update(config))
            .map_err(|err| err.to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        self.command_tx
            .send(WorkerCommand::Stop)
            .map_err(|err| err.to_string())
    }

    pub fn shutdown(&self) -> Result<(), String> {
        self.command_tx
            .send(WorkerCommand::Shutdown)
            .map_err(|err| err.to_string())
    }

    pub fn try_recv(&self) -> Result<WorkerEvent, mpsc::TryRecvError> {
        self.event_rx.try_recv()
    }
}

pub fn spawn_worker() -> WorkerHandle {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    thread::spawn(move || worker_main(command_rx, event_tx));

    WorkerHandle {
        command_tx,
        event_rx,
    }
}

fn worker_main(command_rx: Receiver<WorkerCommand>, event_tx: Sender<WorkerEvent>) {
    let mut backend = match PlatformBackend::new() {
        Ok(backend) => {
            let _ = event_tx.send(WorkerEvent::BackendReady(backend.backend_name().to_owned()));
            backend
        }
        Err(message) => {
            let _ = event_tx.send(WorkerEvent::Error(message));
            return;
        }
    };

    let mut active_config: Option<MotionConfig> = None;
    let mut next_run_at: Option<Instant> = None;

    loop {
        let command = match next_run_at {
            Some(deadline) => {
                let now = Instant::now();
                if deadline <= now {
                    if let Some(config) = active_config {
                        match perform_cycle(&mut backend, config) {
                            Ok(message) => {
                                let _ = event_tx.send(WorkerEvent::Info(message));
                                next_run_at = Some(Instant::now() + config.interval);
                            }
                            Err(message) => {
                                let _ = backend.set_keep_awake(false);
                                active_config = None;
                                next_run_at = None;
                                let _ = event_tx.send(WorkerEvent::RunningChanged(false));
                                let _ = event_tx.send(WorkerEvent::Error(message));
                            }
                        }
                    }
                    continue;
                }

                let timeout = deadline.saturating_duration_since(now);
                match command_rx.recv_timeout(timeout) {
                    Ok(command) => Some(command),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => Some(WorkerCommand::Shutdown),
                }
            }
            None => match command_rx.recv() {
                Ok(command) => Some(command),
                Err(_) => Some(WorkerCommand::Shutdown),
            },
        };

        let Some(command) = command else {
            continue;
        };

        match command {
            WorkerCommand::Start(config) => match backend.set_keep_awake(true) {
                Ok(()) => {
                    active_config = Some(config);
                    next_run_at = Some(Instant::now() + config.interval);
                    let _ = event_tx.send(WorkerEvent::RunningChanged(true));
                    let _ = event_tx.send(WorkerEvent::Info(
                        "已启动：将在下一个间隔触发真实鼠标轨迹。".to_owned(),
                    ));
                }
                Err(message) => {
                    let _ = event_tx.send(WorkerEvent::RunningChanged(false));
                    let _ = event_tx.send(WorkerEvent::Error(message));
                }
            },
            WorkerCommand::Update(config) => {
                if active_config.is_some() {
                    active_config = Some(config);
                    next_run_at = Some(Instant::now() + config.interval);
                    let _ = event_tx.send(WorkerEvent::Info(
                        "参数已更新，新的间隔与轨迹配置将在下一轮生效。".to_owned(),
                    ));
                }
            }
            WorkerCommand::Stop => {
                active_config = None;
                next_run_at = None;
                match backend.set_keep_awake(false) {
                    Ok(()) => {
                        let _ = event_tx.send(WorkerEvent::RunningChanged(false));
                        let _ = event_tx.send(WorkerEvent::Info(
                            "已停止后台任务，并释放防休眠。".to_owned(),
                        ));
                    }
                    Err(message) => {
                        let _ = event_tx.send(WorkerEvent::RunningChanged(false));
                        let _ = event_tx.send(WorkerEvent::Error(message));
                    }
                }
            }
            WorkerCommand::Shutdown => {
                let _ = backend.set_keep_awake(false);
                break;
            }
        }
    }
}
