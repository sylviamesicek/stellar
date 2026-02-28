use std::io::Write;
use winit::event_loop::EventLoop;

mod app;
mod renderer;

fn main() -> eyre::Result<()> {
    // Initialize pretty error handling
    color_eyre::install()?;
    // Initialize logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Always)
        .format(move |buf, record| writeln!(buf, "[{}]: {}", record.level(), record.args()))
        .init();
    // Create the event loop and run the app
    let event_loop = EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = app::App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
