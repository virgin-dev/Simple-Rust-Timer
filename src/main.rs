use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager, GlobalHotKeyEvent};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::mpsc;

struct TimerApp {
    timer_active: Arc<Mutex<bool>>,
    time_left: Arc<Mutex<u64>>,
    sender: mpsc::Sender<String>,
    receiver: mpsc::Receiver<String>,
    hotkey_receiver: mpsc::Receiver<String>,
    display_text: String,
    always_on_top: bool,
}

impl TimerApp {
    fn new(hotkey_receiver: mpsc::Receiver<String>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let timer_active = Arc::new(Mutex::new(false));
        let time_left = Arc::new(Mutex::new(0));

        TimerApp {
            timer_active,
            time_left,
            sender,
            receiver,
            hotkey_receiver,
            display_text: "Нажмите F1 для запуска таймера".to_string(),
            always_on_top: true,
        }
    }

    fn start_timer(&self, timeout: u64) {
        let sender = self.sender.clone();
        let timer_active = Arc::clone(&self.timer_active);
        let time_left = Arc::clone(&self.time_left);

        thread::spawn(move || {
            for i in (0..=timeout).rev() {
                let _ = sender.send(format!("Осталось: {} сек", i));
                *time_left.lock().unwrap() = i;
                thread::sleep(Duration::from_secs(1));
            }

            let _ = sender.send("Таймер завершён!".to_string());
            *timer_active.lock().unwrap() = false;
        });
    }
}

impl eframe::App for TimerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.always_on_top {
            frame.set_always_on_top(true);
        }

        if let Ok(message) = self.receiver.try_recv() {
            self.display_text = message;
        }

        if let Ok(message) = self.hotkey_receiver.try_recv() {
            println!("{}", message);
            let mut active = self.timer_active.lock().unwrap();
            if !*active {
                *active = true;
                self.start_timer(10);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Таймер by_virgin_dev");

            ui.label(
                egui::RichText::new(&self.display_text)
                    .size(40.0)
                    .strong(),
            );

            if ui.button("Запустить таймер").clicked() {
                let mut active = self.timer_active.lock().unwrap();
                if !*active {
                    *active = true;
                    self.start_timer(10);
                }
            }

            ui.separator();
            ui.checkbox(&mut self.always_on_top, "Окно всегда сверху");
        });

        ctx.request_repaint();
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(300.0, 150.0)),
        ..Default::default()
    };

    let hotkey_manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(None, global_hotkey::hotkey::Code::F1);
    hotkey_manager.register(hotkey).unwrap();

    let (hotkey_sender, hotkey_receiver) = mpsc::channel();

    thread::spawn(move || {
        loop {
            if let Some(event) = GlobalHotKeyEvent::receiver().try_recv().ok() {
                if event.id == hotkey.id() {
                    let _ = hotkey_sender.send("F1 нажата!".to_string());
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    eframe::run_native(
        "Таймер",
        options,
        Box::new(|_cc| Box::new(TimerApp::new(hotkey_receiver))),
    );
}