use std::process::exit;

use bindings::{exports::wasi, wasi::windowing::window::Window};

fn main() {
    println!("Main");
    let w = Window::new();
    w.set_visible(true);
    unsafe { COMPONENT.w = Some(w) };
}

mod bindings;

struct Component {
    w: Option<Window>,
}

static mut COMPONENT: Component = Component {
    w: None,
};


impl wasi::windowing::event_handler::Guest for Component {
    fn event_handler(window_id: wasi::windowing::event_handler::WindowId,event: wasi::windowing::event_handler::Event,) {
        println!("Event received!");
        match event {
            bindings::wasi::windowing::event::Event::Close => exit(0),
            bindings::wasi::windowing::event::Event::KeyDown(_) => todo!(),
            bindings::wasi::windowing::event::Event::KeyUp(_) => todo!(),
            bindings::wasi::windowing::event::Event::ClickDown(_) => todo!(),
            bindings::wasi::windowing::event::Event::ClickUp(_) => todo!(),
            bindings::wasi::windowing::event::Event::Move(_) => todo!(),
        }
    }
}


