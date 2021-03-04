#![warn(rust_2018_idioms)]
use futures::stream::StreamExt;
// use serde_json;
use shiplift::rep::Stats;
use shiplift::Docker;
use std::env;

// refer to https://github.com/docker/cli/blob/master/cli/command/container/stats_helpers.go

fn calc_cpu(stat: &Stats, prev_stat: &Stats) -> f64 {
    let prev_cpu = prev_stat.cpu_stats.cpu_usage.total_usage;
    let prev_sys = prev_stat.cpu_stats.system_cpu_usage;

    let cpu = (stat.cpu_stats.cpu_usage.total_usage - prev_cpu) as f64;
    let sys = (stat.cpu_stats.system_cpu_usage - prev_sys) as f64;
    let n_cpus = stat.cpu_stats.cpu_usage.percpu_usage.len() as f64;
    (cpu / sys) * n_cpus * 100.0
}

fn calc_mem(stat: &Stats) -> u64 {
    let v = stat.memory_stats.stats.inactive_file;
    let mu = stat.memory_stats.usage;
    if v < mu {
        mu - v
    } else {
        mu
    }
}

fn calc_cumulative_net(stat: &Stats) -> (u64, u64) {
    let mut total_rx = 0;
    let mut total_tx = 0;

    for (_, net) in &stat.networks {
        total_rx += net.rx_bytes;
        total_tx += net.tx_bytes;
    }
    (total_rx, total_tx)
}

fn calc_net(stat: &Stats, prev_stat: &Stats) -> (u64, u64) {
    let (curr_rx, curr_tx) = calc_cumulative_net(stat);
    let (prev_rx, prev_tx) = calc_cumulative_net(prev_stat);
    ((curr_rx - prev_rx), (curr_tx - prev_tx))
}

fn calc_cumulative_disk(stat: &Stats) -> (u64, u64) {
    let mut read_bytes = 0;
    let mut write_bytes = 0;

    for iostat in &stat.blkio_stats.io_service_bytes_recursive {
        if iostat.op == "Read" {
            read_bytes += iostat.value;
        } else if iostat.op == "Write" {
            write_bytes += iostat.value;
        }
    }
    (read_bytes, write_bytes)
}

fn calc_disk(stat: &Stats, prev_stat: &Stats) -> (u64, u64) {
    let (curr_read_bytes, curr_write_bytes) = calc_cumulative_disk(stat);
    let (prev_read_bytes, prev_write_bytes) = calc_cumulative_disk(prev_stat);
    (
        (curr_read_bytes - prev_read_bytes),
        (curr_write_bytes - prev_write_bytes),
    )
}

fn handle_stat(stat: &Stats, prev_stat: &Stats) {
    //
    let cpu = calc_cpu(stat, &prev_stat);
    let mem = calc_mem(stat) as f64 * 0.0000009536743; // MiB
    let (rx, tx) = calc_net(stat, &prev_stat);
    let (r, w) = calc_disk(stat, &prev_stat);
    println!("{},{},{},{},{},{}", cpu, mem, rx, tx, r, w);
    // let j = serde_json::to_string_pretty(&stat).unwrap();
    // println!("{}", j);
}

fn print_header() {
    println!("cpu,mem,rx,tx,read,write");
}

#[tokio::main]
async fn main() {
    let mut prev_stat: Option<Stats> = None;

    let docker = Docker::new();
    let containers = docker.containers();
    let id = env::args().nth(1).expect("Usage: docker-stats <container>");

    print_header();
    while let Some(result) = containers.get(&id).stats().next().await {
        match result {
            Ok(stat) => {
                if let Some(p) = prev_stat {
                    handle_stat(&stat, &p);
                }
                prev_stat = Some(stat);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1)
            }
        }
    }
}
