#[macro_use]
extern crate rglua;

pub mod server;
pub mod types;

use crate::server::{start_metrics_server, stop_metrics_server};
use crate::types::get_table;
use lazy_static::lazy_static;
use log::{error, info};
use once_cell::sync::Lazy;
use prometheus::{IntCounterVec, Opts, Registry};
use rglua::lua::{
    LuaState, luaL_checkinteger,
    luaL_checkstring, lua_newtable, lua_pushcfunction, lua_setfield, lua_setglobal,
};
use rglua::prelude::luaL_error;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::runtime::Runtime;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref COUNTERS: Mutex<HashMap<String, IntCounterVec>> = Mutex::new(HashMap::new());
    static ref GAUGES: Mutex<HashMap<String, prometheus::GaugeVec>> = Mutex::new(HashMap::new());
    static ref METRICS_SERVER: Mutex<Option<MetricsServer>> = Mutex::new(None);
}

struct MetricsServer {
    thread: JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
}

pub static RT: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("gmsv-exporter-worker")
            .build()
            .expect("Failed to initialize runtime."),
    )
});

fn extract_labels(table: &str) -> Vec<&str> {
    if table.is_empty() {
        vec![]
    } else {
        table.split(',').map(|s| s.trim()).collect()
    }
}

#[lua_function]
pub extern "C" fn prometheus_create_counter(l: LuaState) -> i32 {
    let name = rstr!(luaL_checkstring(l, 1));
    let label_table = rstr!(luaL_checkstring(l, 2));

    let labels = extract_labels(&label_table);

    let opts = Opts::new(format!("gmsv_{}", name), "Lua Prometheus counter");
    let counter = IntCounterVec::new(opts, &labels)
        .unwrap_or_else(|e| panic!("Failed to create counter: {}", e));

    if REGISTRY.register(Box::new(counter.clone())).is_ok() {
        COUNTERS.lock().unwrap().insert(name.to_string(), counter);
    }

    0
}

#[lua_function]
pub extern "C" fn prometheus_create_gauge(l: LuaState) -> i32 {
    let name = rstr!(luaL_checkstring(l, 1));
    let label_table = rstr!(luaL_checkstring(l, 2));

    let labels = extract_labels(&label_table);

    let opts = prometheus::Opts::new(format!("gmsv_{}", name), "Lua Prometheus gauge");
    let gauge = prometheus::GaugeVec::new(opts, &labels)
        .unwrap_or_else(|e| panic!("Failed to create gauge: {}", e));

    if REGISTRY.register(Box::new(gauge.clone())).is_ok() {
        GAUGES.lock().unwrap().insert(name.to_string(), gauge);
    }

    0
}

#[lua_function]
pub extern "C" fn prometheus_increase_counter(l: LuaState) -> i32 {
    let name = rstr!(luaL_checkstring(l, 1));
    let labels_map = unsafe { get_table(l, 2) };
    let value = luaL_checkinteger(l, 3);

    RT.spawn(async move {
        let counters = COUNTERS.lock().unwrap();
        let counter = counters.get(name);

        let mut entries: Vec<_> = labels_map.into_iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        let labels: Vec<String> = entries.into_iter().map(|(_, v)| v).collect();

        if let Some(counter) = counter {
            counter.with_label_values(&labels).inc_by(value as u64);
        } else {
            error!("Counter {} not found", name);
        }
    });

    0
}

#[lua_function]
pub extern "C" fn prometheus_reset(l: LuaState) -> i32 {
    let name = rstr!(luaL_checkstring(l, 1));

    let counters = COUNTERS.lock().unwrap();
    let counter = counters.get(name).unwrap_or_else(|| {
        luaL_error(l, cstr!("Counter not found"));
    });

    counter.reset();

    0
}

#[lua_function]
pub extern "C" fn prometheus_gauge_set(l: LuaState) -> i32 {
    let name = rstr!(luaL_checkstring(l, 1));
    let labels_map = unsafe { get_table(l, 2) };
    let value = luaL_checkinteger(l, 3);

    RT.spawn(async move {
        let gauges = GAUGES.lock().unwrap();
        let gauge = gauges.get(name);

        let mut entries: Vec<_> = labels_map.into_iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        let labels: Vec<String> = entries.into_iter().map(|(_, v)| v).collect();

        if let Some(gauge) = gauge {
            gauge.with_label_values(&labels).set(value as f64);
        } else {
            error!("Gauge {} not found", name);
        }
    });

    0
}

#[lua_function]
pub extern "C" fn prometheus_start_server(l: LuaState) -> i32 {
    let addr = rstr!(luaL_checkstring(l, 1));

    if METRICS_SERVER.lock().unwrap().is_some() {
        stop_metrics_server();
    }

    start_metrics_server(addr.to_string());
    0
}

#[lua_function]
pub extern "C" fn prometheus_stop_server(l: LuaState) -> i32 {
    stop_metrics_server();
    0
}

#[gmod_open]
unsafe fn open(l: LuaState) -> i32 {
    let _ = tracing_subscriber::fmt::try_init();

    lua_newtable(l);
    lua_pushcfunction(l, prometheus_create_counter);
    lua_setfield(l, -2, cstr!("CreateCounter"));
    lua_pushcfunction(l, prometheus_increase_counter);
    lua_setfield(l, -2, cstr!("IncreaseCounter"));
    lua_pushcfunction(l, prometheus_reset);
    lua_setfield(l, -2, cstr!("ResetCounter"));
    lua_pushcfunction(l, prometheus_create_gauge);
    lua_setfield(l, -2, cstr!("CreateGauge"));
    lua_pushcfunction(l, prometheus_gauge_set);
    lua_setfield(l, -2, cstr!("SetGauge"));
    lua_pushcfunction(l, prometheus_start_server);
    lua_setfield(l, -2, cstr!("StartServer"));
    lua_setglobal(l, cstr!("Prometheus"));

    0
}

#[gmod_close]
fn close(_l: LuaState) -> i32 {
    stop_metrics_server();
    0
}
