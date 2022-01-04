use kompact::prelude::*;

#[derive(ComponentDefinition, Actor)]
pub struct HelloWorldComponent {
    ctx: ComponentContext<Self>
}

impl HelloWorldComponent {
    pub fn new() -> HelloWorldComponent {
        HelloWorldComponent {
            ctx: ComponentContext::uninitialised()
        }
    }
}

impl ComponentLifecycle for HelloWorldComponent {
    fn on_start(&mut self) -> Handled {
        info!(self.ctx.log(), "Hello World!");
        self.ctx().system().shutdown_async();
        Handled::Ok
    }
}