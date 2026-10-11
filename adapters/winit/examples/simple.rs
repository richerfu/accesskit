#[path = "util/fill.rs"]
mod fill;

use accesskit::{
    Action, ActionRequest, Affine, Live, Node, NodeId, Rect, Role, TreeId, TreeInfo, TreeUpdate,
    Vec2,
};
use accesskit_winit::{Adapter, Event as AccessKitEvent, WindowEvent as AccessKitWindowEvent};
use std::error::Error;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ControlFlow,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::Key,
    window::{Window, WindowAttributes, WindowId},
};

const WINDOW_TITLE: &str = "Hello world";

const WINDOW_ID: NodeId = NodeId(0);
const BUTTON_1_ID: NodeId = NodeId(1);
const BUTTON_2_ID: NodeId = NodeId(2);
const ANNOUNCEMENT_ID: NodeId = NodeId(3);
const INITIAL_FOCUS: NodeId = BUTTON_1_ID;

const WINDOW_RECT: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 393.0,
    y1: 759.0,
};

const BUTTON_1_RECT: Rect = Rect {
    x0: 20.0,
    y0: 20.0,
    x1: 200.0,
    y1: 64.0,
};

const BUTTON_2_RECT: Rect = Rect {
    x0: 20.0,
    y0: 84.0,
    x1: 200.0,
    y1: 128.0,
};

fn safe_area_inset(window: &dyn Window) -> Vec2 {
    let insets = window.safe_area();
    Vec2::new(f64::from(insets.left), f64::from(insets.top))
}

fn build_button(id: NodeId, label: &str) -> Node {
    let rect = match id {
        BUTTON_1_ID => BUTTON_1_RECT,
        BUTTON_2_ID => BUTTON_2_RECT,
        _ => unreachable!(),
    };
    let mut node = Node::new(Role::Button);
    node.set_bounds(rect);
    node.set_label(label);
    node.add_action(Action::Focus);
    node.add_action(Action::Click);
    node
}

fn build_announcement(text: &str) -> Node {
    let mut node = Node::new(Role::Label);
    node.set_value(text);
    node.set_live(Live::Polite);
    node
}

const ANNOUNCEMENT_DELAY: Duration = Duration::from_millis(150);

struct UiState {
    focus: NodeId,
    announcement: Option<&'static str>,
    pending_announcement: Option<(&'static str, Instant)>,
    scale_factor: f64,
    inset: Vec2,
}

impl UiState {
    fn new(scale_factor: f64, inset: Vec2) -> Self {
        Self {
            focus: INITIAL_FOCUS,
            announcement: None,
            pending_announcement: None,
            scale_factor,
            inset,
        }
    }

    fn build_root(&mut self) -> Node {
        let mut node = Node::new(Role::Window);
        node.set_bounds(WINDOW_RECT);
        node.set_transform(Affine::translate(self.inset) * Affine::scale(self.scale_factor));
        node.set_children(vec![BUTTON_1_ID, BUTTON_2_ID]);
        if self.announcement.is_some() {
            node.push_child(ANNOUNCEMENT_ID);
        }
        node.set_label(WINDOW_TITLE);
        node
    }

    fn build_initial_tree(&mut self) -> TreeUpdate {
        let root = self.build_root();
        let button_1 = build_button(BUTTON_1_ID, "Button 1");
        let button_2 = build_button(BUTTON_2_ID, "Button 2");
        let tree = TreeInfo::new(WINDOW_ID);
        let mut result = TreeUpdate {
            nodes: vec![
                (WINDOW_ID, root),
                (BUTTON_1_ID, button_1),
                (BUTTON_2_ID, button_2),
            ],
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus: self.focus,
        };
        if let Some(announcement) = &self.announcement {
            result
                .nodes
                .push((ANNOUNCEMENT_ID, build_announcement(announcement)));
        }
        result
    }

    fn set_focus(&mut self, adapter: &mut Adapter, focus: NodeId) {
        self.focus = focus;
        adapter.update_if_active(|| TreeUpdate {
            nodes: vec![],
            tree: None,
            tree_id: TreeId::ROOT,
            focus,
        });
    }

    fn press_button(&mut self, id: NodeId) {
        let text = if id == BUTTON_1_ID {
            "You pressed button 1"
        } else {
            "You pressed button 2"
        };
        // On iOS, VoiceOver announces the label of the activated button.
        // Postpone the live region update so the messages don't overlap.
        self.pending_announcement = Some((text, Instant::now()));
    }

    fn flush_announcement(&mut self, adapter: &mut Adapter) -> bool {
        let Some((_, queued_at)) = &self.pending_announcement else {
            return false;
        };
        if queued_at.elapsed() < ANNOUNCEMENT_DELAY {
            return true;
        }
        if let Some((text, _)) = self.pending_announcement.take() {
            self.announcement = Some(text);
            adapter.update_if_active(|| {
                let announcement = build_announcement(text);
                let root = self.build_root();
                TreeUpdate {
                    nodes: vec![(ANNOUNCEMENT_ID, announcement), (WINDOW_ID, root)],
                    tree: None,
                    tree_id: TreeId::ROOT,
                    focus: self.focus,
                }
            });
        }
        false
    }
}

struct WindowState {
    window: Box<dyn Window>,
    adapter: Adapter,
    ui: UiState,
}

impl WindowState {
    fn new(window: Box<dyn Window>, adapter: Adapter, ui: UiState) -> Self {
        Self {
            window,
            adapter,
            ui,
        }
    }
}

struct Application {
    event_handler: std::sync::Arc<dyn Fn(AccessKitEvent) + Send + Sync>,
    events: std::sync::mpsc::Receiver<AccessKitEvent>,
    window: Option<WindowState>,
}

impl Application {
    fn new(
        event_handler: std::sync::Arc<dyn Fn(AccessKitEvent) + Send + Sync>,
        events: std::sync::mpsc::Receiver<AccessKitEvent>,
    ) -> Self {
        Self {
            event_handler,
            events,
            window: None,
        }
    }

    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes = WindowAttributes::default()
            .with_title(WINDOW_TITLE)
            .with_visible(false);

        let window = event_loop.create_window(window_attributes)?;
        let ui = UiState::new(window.scale_factor(), safe_area_inset(window.as_ref()));
        let adapter =
            Adapter::with_event_handler(event_loop, window.as_ref(), self.event_handler.clone());
        window.set_visible(true);

        self.window = Some(WindowState::new(window, adapter, ui));
        Ok(())
    }
}

impl ApplicationHandler for Application {
    fn window_event(&mut self, _: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = match &mut self.window {
            Some(window) => window,
            None => return,
        };
        let adapter = &mut window.adapter;
        let state = &mut window.ui;

        adapter.process_event(window.window.as_ref(), &event);
        match event {
            WindowEvent::CloseRequested => {
                fill::cleanup_window(window.window.as_ref());
                self.window = None;
            }
            WindowEvent::SurfaceResized(_) => {
                state.scale_factor = window.window.scale_factor();
                state.inset = safe_area_inset(window.window.as_ref());
                adapter.update_if_active(|| state.build_initial_tree());
                window.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                fill::fill_window(window.window.as_ref());
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: virtual_code,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match virtual_code {
                Key::Named(winit::keyboard::NamedKey::Tab) => {
                    let new_focus = if state.focus == BUTTON_1_ID {
                        BUTTON_2_ID
                    } else {
                        BUTTON_1_ID
                    };
                    state.set_focus(adapter, new_focus);
                    window.window.request_redraw();
                }
                Key::Character(text) if text == " " => {
                    let id = state.focus;
                    state.press_button(id);
                    window.window.request_redraw();
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.events.try_recv() {
            self.accesskit_event(event_loop, event);
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_none() {
            self.create_window(event_loop)
                .expect("failed to create initial window");
        }
        if let Some(window) = self.window.as_ref() {
            window.window.request_redraw();
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        if let Some(window) = self.window.take() {
            fill::cleanup_window(window.window.as_ref());
        }
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        if let Some(window) = &mut self.window {
            if window.ui.flush_announcement(&mut window.adapter) {
                event_loop.set_control_flow(ControlFlow::wait_duration(ANNOUNCEMENT_DELAY));
            }
        } else {
            event_loop.exit();
        }
    }
}

impl Application {
    fn accesskit_event(&mut self, _: &dyn ActiveEventLoop, user_event: AccessKitEvent) {
        let window = match &mut self.window {
            Some(window) => window,
            None => return,
        };
        let adapter = &mut window.adapter;
        let state = &mut window.ui;

        match user_event.window_event {
            AccessKitWindowEvent::InitialTreeRequested => {
                adapter.update_if_active(|| state.build_initial_tree());
            }
            AccessKitWindowEvent::ActionRequested(ActionRequest {
                action,
                target_node,
                ..
            }) => {
                if target_node == BUTTON_1_ID || target_node == BUTTON_2_ID {
                    match action {
                        Action::Focus => {
                            state.set_focus(adapter, target_node);
                        }
                        Action::Click => {
                            state.press_button(target_node);
                        }
                        _ => (),
                    }
                }
                window.window.request_redraw();
            }
            AccessKitWindowEvent::AccessibilityDeactivated => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("This example has no visible GUI, and a keyboard interface:");
    println!("- [Tab] switches focus between two logical buttons.");
    println!(
        "- [Space] 'presses' the button, adding static text in a live region announcing that it was pressed."
    );
    #[cfg(target_os = "windows")]
    println!(
        "Enable Narrator with [Win]+[Ctrl]+[Enter] (or [Win]+[Enter] on older versions of Windows)."
    );
    #[cfg(all(
        feature = "accesskit_unix",
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    ))]
    println!("Enable Orca with [Super]+[Alt]+[S].");

    let event_loop = EventLoop::new()?;
    let proxy = event_loop.create_proxy();
    let (sender, receiver) = std::sync::mpsc::channel();
    let handler = std::sync::Arc::new(move |event| {
        if sender.send(event).is_ok() {
            proxy.wake_up();
        }
    });
    let state = Application::new(handler, receiver);
    event_loop.run_app(state).map_err(Into::into)
}
