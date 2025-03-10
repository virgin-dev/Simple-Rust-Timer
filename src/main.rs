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
    timer_duration: u64,
    hotkey: Arc<Mutex<HotKey>>,
    hotkey_manager: Arc<Mutex<GlobalHotKeyManager>>,
    last_update_time: Option<f64>,
}

impl TimerApp {
    fn new(hotkey_receiver: mpsc::Receiver<String>, hotkey_manager: Arc<Mutex<GlobalHotKeyManager>>, hotkey: Arc<Mutex<HotKey>>) -> Self {
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
            timer_duration: 10,
            hotkey,
            hotkey_manager,
            last_update_time: None,
        }
    }

    fn start_timer(&mut self) {
        *self.timer_active.lock().unwrap() = true;
        *self.time_left.lock().unwrap() = self.timer_duration;
        self.last_update_time = None;
    }

    fn stop_timer(&mut self) {
        *self.timer_active.lock().unwrap() = false; // Останавливаем таймер
        self.display_text = "Таймер остановлен!".to_string();
    }

    fn set_hotkey(&mut self, key: global_hotkey::hotkey::Code) {
        let old_hotkey = self.hotkey.lock().unwrap().clone();
        self.hotkey_manager.lock().unwrap().unregister(old_hotkey).unwrap();

        let new_hotkey = HotKey::new(None, key);
        self.hotkey_manager.lock().unwrap().register(new_hotkey.clone()).unwrap();

        *self.hotkey.lock().unwrap() = new_hotkey;

        self.display_text = format!("Горячая клавиша изменена на {:?}", key);
    }

    fn update_timer(&mut self, current_time: f64) {
        if *self.timer_active.lock().unwrap() {
            if let Some(last_time) = self.last_update_time {
                let delta_time = current_time - last_time;
                if delta_time >= 1.0 {
                    let mut time_left = self.time_left.lock().unwrap();
                    if *time_left > 0 {
                        *time_left -= 1;
                        self.display_text = format!("Осталось: {} сек", *time_left);
                    } else {
                        *self.timer_active.lock().unwrap() = false;
                        self.display_text = "Таймер завершён!".to_string();
                    }
                    self.last_update_time = Some(current_time);
                }
            } else {
                self.last_update_time = Some(current_time);
            }
        }
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
            let should_start = {
                let active = self.timer_active.lock().unwrap();
                !*active
            };
            if should_start {
                self.start_timer();
            }
        }

        let current_time = ctx.input(|i| i.time); // Получаем текущее время
        self.update_timer(current_time);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Таймер");

            ui.label(
                egui::RichText::new(&self.display_text)
                    .size(40.0) // Увеличиваем размер текста
                    .strong(), // Делаем текст жирным
            );

            ui.horizontal(|ui| {
                ui.label("Длительность таймера (сек):");
                ui.add(egui::DragValue::new(&mut self.timer_duration).speed(1));
            });

            if ui.button("Запустить таймер").clicked() {
                let should_start = {
                    let active = self.timer_active.lock().unwrap();
                    !*active
                };
                if should_start {
                    self.start_timer();
                }
            }

            if ui.button("Остановить таймер").clicked() {
                self.stop_timer();
            }

            ui.separator();
            ui.checkbox(&mut self.always_on_top, "Окно всегда сверху");

            ui.horizontal(|ui| {
                ui.label("Горячая клавиша:");
                if ui.button("F1").clicked() {
                    self.set_hotkey(global_hotkey::hotkey::Code::F1);
                }
                if ui.button("F2").clicked() {
                    self.set_hotkey(global_hotkey::hotkey::Code::F2);
                }
                if ui.button("F3").clicked() {
                    self.set_hotkey(global_hotkey::hotkey::Code::F3);
                }
            });
        });

        ctx.request_repaint();
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(400.0, 200.0)), // Заранее заданные размеры окна
        ..Default::default()
    };

    let hotkey_manager = Arc::new(Mutex::new(GlobalHotKeyManager::new().unwrap()));
    let hotkey = Arc::new(Mutex::new(HotKey::new(None, global_hotkey::hotkey::Code::F1))); // Горячая клавиша F1

    hotkey_manager.lock().unwrap().register(hotkey.lock().unwrap().clone()).unwrap();

    let (hotkey_sender, hotkey_receiver) = mpsc::channel();

    let hotkey_manager_clone = Arc::clone(&hotkey_manager);
    let hotkey_clone = Arc::clone(&hotkey);
    thread::spawn(move || {
        loop {
            if let Some(event) = GlobalHotKeyEvent::receiver().try_recv().ok() {
                if event.id == hotkey_clone.lock().unwrap().id() {
                    let _ = hotkey_sender.send("Горячая клавиша нажата!".to_string());
                }
            }
            thread::sleep(Duration::from_millis(100)); // Небольшая задержка для снижения нагрузки на CPU
        }
    });

    eframe::run_native(
        "Таймер",
        options,
        Box::new(|_cc| Box::new(TimerApp::new(hotkey_receiver, hotkey_manager_clone, hotkey))),
    );
}