use gcfeeder::feeder;

use druid::widget::{Button, Flex, Label};
use druid::{Data, Env, Widget, WidgetExt};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use feeder::Feeder;
use thread::JoinHandle;

pub struct FeederThread {
    handle: Option<JoinHandle<()>>,
    sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<feeder::Error>,
}

impl FeederThread {
    pub fn spawn() -> Result<FeederThread, feeder::Error> {
        let (terminate_send, terminate_recv) = mpsc::channel();
        let (error_send, error_recv) = mpsc::channel();

        let mut feeder = Feeder::new()?;

        let handle = thread::spawn(move || loop {
            if let Ok(_) = terminate_recv.try_recv() {
                break;
            }

            if let Err(err) = feeder.feed() {
                error_send
                    .send(err)
                    .expect("failed to send error from feeder thread");
                break;
            }

            thread::sleep(Duration::from_millis(2));
        });

        Ok(FeederThread {
            handle: Some(handle),
            sender: terminate_send,
            receiver: error_recv,
        })
    }

    pub fn error(&self) -> Option<feeder::Error> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for FeederThread {
    fn drop(&mut self) {
        self.sender
            .send(())
            .expect("failed to terminate feeder thread");

        if let Some(handle) = self.handle.take() {
            handle.join().expect("failed to join feeder thread");
        }
    }
}

#[derive(Clone, Data)]
pub struct AppState {
    feeder_thread: Option<Rc<FeederThread>>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            feeder_thread: None,
        }
    }
}

pub fn builder() -> impl Widget<AppState> {
    let label = Label::new(|state: &AppState, _env: &Env| {
        if state.feeder_thread.is_some() {
            "Running"
        } else {
            "Not Running"
        }
        .into()
    })
    .padding(5.0)
    .center();

    let start_button = Button::new("Start")
        .on_click(|_ctx, state: &mut AppState, _env| {
            if state.feeder_thread.is_none() {
                state.feeder_thread = Some(Rc::new(FeederThread::spawn().unwrap()));
            }
        })
        .padding(5.0);

    let stop_button = Button::new("Stop")
        .on_click(|_ctx, state: &mut AppState, _env| {
            if state.feeder_thread.is_some() {
                state.feeder_thread = None;
            }
        })
        .padding(5.0);

    Flex::column()
        .with_child(label)
        .with_child(start_button)
        .with_child(stop_button)
}
