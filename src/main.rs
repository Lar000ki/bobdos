use eframe::egui;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use tokio::task;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "BOBDOS",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

#[derive(Default, Clone)]
struct MyApp {
    url: String,
    num_requests: String,
    timeout_duration: String,
    public_ip: Arc<Mutex<String>>,
    result: Arc<Mutex<String>>,
    loading: Arc<Mutex<bool>>, 
    success_count: Arc<Mutex<usize>>, 
    failure_count: Arc<Mutex<usize>>,
}

async fn get_public_ip(client: &Client) -> Result<String, reqwest::Error> {
    let response = client.get("https://api.ipify.org").send().await?;
    let ip: String = response.text().await?;
    Ok(ip)
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BOBDOS");

            ui.label("URL:");
            ui.text_edit_singleline(&mut self.url);

            ui.label("number of requests:");
            ui.text_edit_singleline(&mut self.num_requests);

            ui.label("timeout (in seconds):");
            ui.text_edit_singleline(&mut self.timeout_duration);

            if ui.button("show IP").clicked() {
                let public_ip_clone = Arc::clone(&self.public_ip);

                tokio::spawn(async move {
                    let client = Client::new();
                    match get_public_ip(&client).await {
                        Ok(ip) => {
                            //println!("IP: {}", ip);
                            *public_ip_clone.lock().unwrap() = ip;
                        }
                        Err(e) => {
                            //eprintln!("show IP error: {}", e);
                            *public_ip_clone.lock().unwrap() = "show IP error".to_string();
                        }
                    }
                });
            }

            let ip_display = self.public_ip.lock().unwrap().clone();
            ui.label(format!("IP: {}", ip_display));

            if *self.loading.lock().unwrap() {
                let spinner = ["|", "/", "-", "\\"];
                let idx = (ctx.input(|i| i.time) / 0.2) as usize % spinner.len();
                ui.label(format!("in progress... {}", spinner[idx]));
                ctx.request_repaint();
            }

            ui.label(format!("successful requests: {}", *self.success_count.lock().unwrap()));
            ui.label(format!("unsuccessful requests: {}", *self.failure_count.lock().unwrap()));

            if ui.button("run with a limited number of requests").clicked() {
                *self.loading.lock().unwrap() = true;
                *self.success_count.lock().unwrap() = 0;
                *self.failure_count.lock().unwrap() = 0;

                let url = self.url.clone();
                let num_requests: usize = self.num_requests.parse().unwrap_or(0);

                let success_count = Arc::clone(&self.success_count);
                let failure_count = Arc::clone(&self.failure_count);
                let result = Arc::clone(&self.result);
                let loading_flag = Arc::clone(&self.loading);

                tokio::spawn(async move {
                    if num_requests > 0 && !url.is_empty() {
                        let client = Client::new();
                        let tasks: Vec<_> = (0..num_requests).map(|_| {
                            let client = client.clone();
                            let url = url.clone();
                            let success_count = Arc::clone(&success_count);
                            let failure_count = Arc::clone(&failure_count);
                            task::spawn(async move {
                                let res = client.get(&url).send().await;

                                match res {
                                    Ok(_) => {
                                        *success_count.lock().unwrap() += 1;
                                    }
                                    Err(_) => {
                                        *failure_count.lock().unwrap() += 1;
                                    }
                                }
                                true
                            })
                        }).collect();

                        for task in tasks {
                            task.await.unwrap_or(false);
                        }
                    } else {
                        *result.lock().unwrap() = "invalid URL or number of requests".to_string();
                    }

                    *loading_flag.lock().unwrap() = false;
                });
            }

            if ui.button("Run for timeout").clicked() {
                *self.loading.lock().unwrap() = true; 

                *self.success_count.lock().unwrap() = 0; 
                *self.failure_count.lock().unwrap() = 0; 

                let url = self.url.clone();
                let timeout_duration: u64 = self.timeout_duration.parse().unwrap_or(0);

                let success_count = Arc::clone(&self.success_count);
                let failure_count = Arc::clone(&self.failure_count);
                let loading_flag = Arc::clone(&self.loading);

                tokio::spawn(async move {
                    let client = Client::new();
                    let start_time = tokio::time::Instant::now();

                    while start_time.elapsed().as_secs() < timeout_duration {
                        let res = client.get(&url).send().await;

                        match res {
                            Ok(_) => {
                                *success_count.lock().unwrap() += 1;
                            }
                            Err(_) => {
                                *failure_count.lock().unwrap() += 1;
                            }
                        }

                        //tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    *loading_flag.lock().unwrap() = false;
                });
            }

            let result_for_ui = Arc::clone(&self.result);
            ui.label(&*result_for_ui.lock().unwrap());
        });
    }
}
