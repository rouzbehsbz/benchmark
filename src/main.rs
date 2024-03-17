use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use clap::Parser;
use reqwest::{self, Proxy};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connections: usize,
    #[arg(short, long)]
    duration: u64,
    #[arg(short, long)]
    proxy: Option<String>,
    #[arg(short, long)]
    target: String,
}

fn main() {
    let args = Args::parse();
    let duration = Duration::from_secs(args.duration);
    let mut clients = Vec::with_capacity(args.connections);
    let mut threads = Vec::new();
    let start_time = Instant::now();

    let ok_statuses = Arc::new(Mutex::new(0));
    let ok_statuses_sum_time = Arc::new(Mutex::new(0));
    let non_ok_statuses = Arc::new(Mutex::new(0));
    let non_ok_statuses_sum_time = Arc::new(Mutex::new(0));

    println!(
        "Benchmarking with {} connection for {}s",
        args.connections, args.duration
    );

    println!("Target: {}", args.target);

    if let Some(ref proxy) = args.proxy {
        println!("Proxy: {}", proxy);
    }

    for _ in 0..args.connections {
        let mut builder = reqwest::blocking::Client::builder()
            .tcp_keepalive(duration)
            .timeout(Duration::from_secs(5));

        let proxy: Option<Proxy> = match args.proxy {
            Some(ref url) => Some(Proxy::all(url).unwrap()),
            None => None,
        };

        match proxy {
            Some(proxy) => builder = builder.proxy(proxy),
            None => {}
        };

        let client = builder.build().unwrap();

        clients.push(client);
    }

    for client in clients {
        let target = args.target.clone();

        let ok_statuses = ok_statuses.clone();
        let ok_statuses_sum_time = ok_statuses_sum_time.clone();
        let non_ok_statuses = non_ok_statuses.clone();
        let non_ok_statuses_sum_time = non_ok_statuses_sum_time.clone();

        let thread = thread::spawn(move || loop {
            let time = Instant::now();
            let time_passed = time - start_time;

            if time_passed.ge(&duration) {
                break;
            }

            let result = client.get(target.clone()).send();
            let res_time = Instant::now() - time;

            match result {
                Ok(result) => {
                    if result.status() == 200 {
                        *ok_statuses.lock().unwrap() += 1;
                        *ok_statuses_sum_time.lock().unwrap() += Duration::as_millis(&res_time);
                    } else {
                        *non_ok_statuses.lock().unwrap() += 1;
                        *non_ok_statuses_sum_time.lock().unwrap() += Duration::as_millis(&res_time);
                    }
                }
                Err(_) => {
                    *non_ok_statuses.lock().unwrap() += 1;
                    *non_ok_statuses_sum_time.lock().unwrap() += Duration::as_millis(&res_time);
                }
            }
        });

        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap()
    }

    let ok_statuses = ok_statuses.lock().unwrap().clone();
    let non_ok_statuses = non_ok_statuses.lock().unwrap().clone();

    let mut ok_statuses_avg_time: f32 = 0.0;

    if ok_statuses != 0 {
        ok_statuses_avg_time =
            ok_statuses_sum_time.lock().unwrap().clone() as f32 / ok_statuses as f32;
    }

    let mut non_ok_statuses_avg_time: f32 = 0.0;

    if non_ok_statuses != 0 {
        non_ok_statuses_avg_time =
            non_ok_statuses_sum_time.lock().unwrap().clone() as f32 / non_ok_statuses as f32;
    }

    println!("{} requests sent.", ok_statuses + non_ok_statuses);
    println!(
        "{} requests with 'OK' status and average of {}ms",
        ok_statuses, ok_statuses_avg_time
    );
    println!(
        "{} requests with other statuses and average of {}ms",
        non_ok_statuses, non_ok_statuses_avg_time
    );
}
