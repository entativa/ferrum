//! gesture.rs — Gesture velocity tracking and recognition.

use crate::types::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum GestureEvent {
    SwipeUp    { velocity: Vec2 },
    SwipeDown  { velocity: Vec2 },
    SwipeLeft  { velocity: Vec2 },
    SwipeRight { velocity: Vec2 },
    Flick      { velocity: Vec2 },
    Drag       { delta: Vec2, velocity: Vec2 },
    Pinch      { scale_delta: f32 },
    LongPress  { position: Vec2 },
}
