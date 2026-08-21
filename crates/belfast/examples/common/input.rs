use belfast::OrbitalPointerButton;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

const LINE_SCROLL_PIXELS: f32 = 16.0;

fn map_button(button: MouseButton) -> Option<OrbitalPointerButton> {
    match button {
        MouseButton::Left => Some(OrbitalPointerButton::Primary),
        MouseButton::Middle => Some(OrbitalPointerButton::Middle),
        _ => None,
    }
}

fn normalize_scroll(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => -y * LINE_SCROLL_PIXELS,
        MouseScrollDelta::PixelDelta(position) => -position.y as f32,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    PointerDown {
        position: [f32; 2],
        button: OrbitalPointerButton,
        pan_modifier: bool,
    },
    PointerMove {
        position: [f32; 2],
    },
    PointerUp {
        button: OrbitalPointerButton,
    },
    Scroll {
        delta: f32,
    },
}

#[derive(Default)]
pub struct InputState {
    cursor_position: [f32; 2],
    shift_pressed: bool,
}

impl InputState {
    pub fn process(&mut self, event: &WindowEvent) -> Option<InputEvent> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = [position.x as f32, position.y as f32];
                Some(InputEvent::PointerMove {
                    position: self.cursor_position,
                })
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_pressed = modifiers.state().shift_key();
                None
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => map_button(*button).map(|button| InputEvent::PointerDown {
                position: self.cursor_position,
                button,
                pan_modifier: self.shift_pressed,
            }),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => map_button(*button).map(|button| InputEvent::PointerUp { button }),
            WindowEvent::MouseWheel { delta, .. } => Some(InputEvent::Scroll {
                delta: normalize_scroll(*delta),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{map_button, normalize_scroll};
    use belfast::OrbitalPointerButton;
    use winit::{
        dpi::PhysicalPosition,
        event::{MouseButton, MouseScrollDelta},
    };

    #[test]
    fn maps_only_supported_pointer_buttons() {
        assert_eq!(
            map_button(MouseButton::Left),
            Some(OrbitalPointerButton::Primary)
        );
        assert_eq!(
            map_button(MouseButton::Middle),
            Some(OrbitalPointerButton::Middle)
        );
        assert_eq!(map_button(MouseButton::Right), None);
    }

    #[test]
    fn normalizes_scroll_to_positive_zoom_out_pixels() {
        assert_eq!(
            normalize_scroll(MouseScrollDelta::LineDelta(0.0, 2.0)),
            -32.0
        );
        assert_eq!(
            normalize_scroll(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                0.0, -24.0
            ))),
            24.0
        );
    }
}
