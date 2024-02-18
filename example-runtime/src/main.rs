use std::{collections::HashMap, env::args, rc::Rc, sync::{mpsc, Arc}, time::{Duration, Instant}};

use exports::wasi::windowing::event_handler::{Event, WindowId};
use tokio::sync::Mutex;
use wasi::windowing::window::HostWindow;
use wasmtime::{component::{bindgen, Component, Linker, Lower, Resource, ResourceTable}, Config, Engine, Store};
use wasmtime_wasi::{preview2::{self, bindings::cli::environment::Host, WasiCtx, WasiView}};
use winit::{dpi::{LogicalPosition, LogicalSize}, event::{DeviceEvent, WindowEvent}, event_loop::{self, EventLoop, EventLoopBuilder}, window::{Window, WindowBuilder}};

use winit;

bindgen!({
    path: "../wit/",
    world: "gui-app-c",
    async: {
        only_imports: [
            "poll"
        ]
    },
    with: {
        "wasi:cli/exit": preview2::bindings::cli::exit,
        "wasi:cli/environment": preview2::bindings::cli::environment,
        "wasi:cli/stdin": preview2::bindings::cli::stdin,
        "wasi:cli/stdout": preview2::bindings::cli::stdout,
        "wasi:cli/stderr": preview2::bindings::cli::stderr,
        "wasi:cli/terminal-input": preview2::bindings::cli::terminal_input,
        "wasi:cli/terminal-output": preview2::bindings::cli::terminal_output,
        "wasi:cli/terminal-stdin": preview2::bindings::cli::terminal_stdin,
        "wasi:cli/terminal-stdout": preview2::bindings::cli::terminal_stdout,
        "wasi:cli/terminal-stderr": preview2::bindings::cli::terminal_stderr,
        "wasi:clocks/monotonic-clock": preview2::bindings::clocks::monotonic_clock,
        "wasi:clocks/wall-clock": preview2::bindings::clocks::wall_clock,
        "wasi:io/error": preview2::bindings::io::error,
        "wasi:io/poll": preview2::bindings::io::poll,
        "wasi:io/streams": preview2::bindings::io::streams,
        "wasi:filesystem/types": preview2::bindings::filesystem::types,
        "wasi:filesystem/preopens": preview2::bindings::filesystem::preopens,
        "wasi:sockets/network": preview2::bindings::sockets::network,
        "wasi:sockets/instance-network": preview2::bindings::sockets::instance_network,
        "wasi:sockets/udp": preview2::bindings::sockets::udp,
        "wasi:sockets/udp-create-socket": preview2::bindings::sockets::udp_create_socket,
        "wasi:sockets/tcp": preview2::bindings::sockets::tcp,
        "wasi:sockets/tcp-create-socket": preview2::bindings::sockets::tcp_create_socket,
        "wasi:sockets/ip-name-lookup": preview2::bindings::sockets::ip_name_lookup,
        "wasi:random/random": preview2::bindings::random::random,
        "wasi:random/insecure": preview2::bindings::random::insecure,
        "wasi:random/insecure-seed": preview2::bindings::random::insecure_seed,
        "wasi:windowing/window/window": winit::window::Window,
    }
});


struct WindowingState {
    ctx: WasiCtx,
    table: ResourceTable,
    window_channel: mpsc::Receiver<winit::window::Window>,
    window_request: mpsc::Sender<()>,
}


impl wasi::windowing::window::Host for WindowingState {
    
}

impl wasi::windowing::event::Host for WindowingState {
    
}

impl WasiView for WindowingState {
    fn table(&self) -> &ResourceTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&self) -> &preview2::WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut preview2::WasiCtx {
        &mut self.ctx
    }
}


impl HostWindow for WindowingState {
    #[doc = " Creates a new window, invisible and of implementation-defined size at an implementation-defined location."]
    fn new(&mut self) -> wasmtime::Result<wasmtime::component::Resource<Window>> {
        self.window_request.send(()).unwrap();
        let w = self.window_channel.recv().unwrap();
        let id = w.id();
        if let Ok(r) = self.table.push(w) {
            return Ok(r);
        } else {
            return Err(wasmtime::Error::msg("Could not create new WASI resource"));
        }
    }

    #[doc = " TODO Add methods to set window parameters and flags."]
    #[doc = " Decide if the methods should return Option<> or false if they aren\\'t supported on the platform, or if the"]
    #[doc = " capabilities should be queryable and the methods are no-ops or return None."]
    #[doc = " Sets the visibility of the window."]
    fn set_visible(&mut self, self_:wasmtime::component::Resource<Window>, visible:bool,) -> wasmtime::Result<()> {
        if let Ok(w) = self.table.get::<winit::window::Window>(&self_) {
            w.set_visible(visible);
            return Ok(());
        } else {
            return Err(wasmtime::Error::msg("Invalid window handle"));
        }
    }

    fn drop(&mut self, rep:wasmtime::component::Resource<Window>) -> wasmtime::Result<()> {
        if let Ok(_) = self.table.delete::<winit::window::Window>(rep) {
            Ok(())
        } else {
            Err(wasmtime::Error::msg("Invalid window handle to drop"))
        }
    }
}



async fn run(inst: Arc<GuiAppC>, store: Arc<Mutex<Store<WindowingState>>>) {
    let mut s = store.lock().await;
    inst.wasi_cli_run().call_run(&mut*s).await.unwrap().unwrap();
}

async fn send_event(inst: Arc<GuiAppC>, store: Arc<Mutex<Store<WindowingState>>>, id: WindowId, event: Event) {
    let mut s = store.lock().await;
    inst.wasi_windowing_event_handler().call_event_handler(&mut* s, id as u64, event).await.unwrap();
}


fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut cfg = Config::new();
    
    cfg.wasm_component_model(true);
    cfg.async_support(true);
    
    let engine = Engine::new(&cfg).unwrap();
    let mut linker = Linker::new(&engine);
    
    let (ws, wr) = std::sync::mpsc::channel();
    let (wrs, wrr) = std::sync::mpsc::channel();
    let mut store = Store::new(&engine, WindowingState {
        ctx: preview2::WasiCtxBuilder::new().inherit_stdout().build(),
        table: ResourceTable::new(),
        window_channel: wr,
        window_request: wrs,
    });
    
    
    
    preview2::bindings::cli::exit::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::environment::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::stdin::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::stdout::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::stderr::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::terminal_input::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::terminal_output::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::terminal_stdin::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::terminal_stdout::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::cli::terminal_stderr::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::clocks::monotonic_clock::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::clocks::wall_clock::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::io::error::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::io::poll::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::io::streams::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::filesystem::types::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::filesystem::preopens::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::network::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::instance_network::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::udp::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::udp_create_socket::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::tcp::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::tcp_create_socket::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::sockets::ip_name_lookup::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::random::random::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::random::insecure::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    preview2::bindings::random::insecure_seed::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    
    wasi::windowing::window::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    wasi::windowing::event::add_to_linker(&mut linker, |state: &mut WindowingState| state).unwrap();
    
    let event_loop = EventLoop::new().unwrap();
    let args: Vec<String> = args().collect();
    let component = Component::from_file(&engine, format!("../example-apps/{}/target/wasm32-wasi/release/{}.wasm", args[1], args[1])).unwrap();
    let (instance, _) = rt.block_on(GuiAppC::instantiate_async(&mut store, &component, &linker)).unwrap();
    let instance = Arc::new(instance);
    let store = Arc::new(Mutex::new(store));
    
    rt.spawn(run(instance.clone(), store.clone()));
    
    
    event_loop.set_control_flow(event_loop::ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(16)));
    event_loop.run(|event, l| {
        match event {
            winit::event::Event::NewEvents(cause) => {
                if wrr.recv_timeout(Duration::ZERO).is_ok() {
                    ws.send(winit::window::WindowBuilder::new().with_position(LogicalPosition::new(100.0, 100.0)).with_inner_size(LogicalSize::new(100.0, 100.0)).with_visible(false).build(l).unwrap()).unwrap();
                }
                match cause {
                    winit::event::StartCause::ResumeTimeReached { start, requested_resume } => {
                        l.set_control_flow(event_loop::ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(16)));
                    },
                    winit::event::StartCause::WaitCancelled { start, requested_resume } => {
                        l.set_control_flow(event_loop::ControlFlow::WaitUntil(requested_resume.unwrap()));
                    },
                    _ => {},
                }
            },
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    WindowEvent::CloseRequested => {
                        //instance.wasi_windowing_event_handler().call_event_handler(&mut store, window_id.into(), wasi::windowing::event::Event::Close);
                        //rt.block_on(instance.wasi_windowing_event_handler().call_event_handler(&mut store, window_id.into(), wasi::windowing::event::Event::Close)).unwrap();
                        rt.block_on(send_event(instance.clone(), store.clone(), window_id.into(), Event::Close));
                    },
                    WindowEvent::KeyboardInput { device_id, event, is_synthetic } => {
                        if ! event.repeat {
                            if let Some(text) = event.text {
                                let code = text.chars().next().unwrap() as u32;
                                let e = match event.state {
                                    winit::event::ElementState::Pressed => wasi::windowing::event::Event::KeyDown(code),
                                    winit::event::ElementState::Released => wasi::windowing::event::Event::KeyUp(code),
                                };
                                rt.block_on(send_event(instance.clone(), store.clone(), window_id.into(), e));
                            }
                        }
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    }).unwrap();
    
    
    
}
