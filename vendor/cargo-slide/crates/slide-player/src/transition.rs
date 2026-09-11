use slide_core::animation::AnimationRegistry;
use slide_core::animation::ComponentAnimation;
use slide_core::animation::RenderContext;
use slide_core::animation::SlideAnimation;
use slide_core::animation::SlideSurface;
use slide_core::model::Rect;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

pub struct TransitionManager {
    registry: AnimationRegistry,
    active_transition: Option<ActiveTransition>,
    active_component_step: Option<ActiveComponentStep>,
}

struct ActiveTransition {
    animation: Arc<dyn SlideAnimation>,
    from: Arc<SlideSurface>,
    to: Arc<SlideSurface>,
    start_time: Instant,
    duration: Duration,
}

struct ActiveComponentStep {
    animation: Arc<dyn ComponentAnimation>,
    rect: Rect,
    start_time: Instant,
    duration: Duration,
}

impl TransitionManager {
    pub fn new(registry: AnimationRegistry) -> Self {
        Self {
            registry,
            active_transition: None,
            active_component_step: None,
        }
    }

    pub fn start_transition(
        &mut self,
        anim_name: &str,
        from: Arc<SlideSurface>,
        to: Arc<SlideSurface>,
    ) {
        let animation = self
            .registry
            .get(anim_name)
            .unwrap_or_else(|| self.registry.get("fade").unwrap());

        let duration = animation.duration();
        if duration.is_zero() {
            self.active_transition = None;
            return;
        }

        self.active_transition = Some(ActiveTransition {
            animation,
            from,
            to,
            start_time: Instant::now(),
            duration,
        });
    }

    pub fn start_component_step(
        &mut self,
        effect_name: &str,
        rect: Rect,
    ) {
        let animation = self
            .registry
            .get_component(effect_name)
            .unwrap_or_else(|| self.registry.get_component("fade-in").unwrap());

        let duration = animation.duration();
        self.active_component_step = Some(ActiveComponentStep {
            animation,
            rect,
            start_time: Instant::now(),
            duration,
        });
    }

    pub fn is_animating(&self) -> bool {
        self.active_transition.is_some() || self.active_component_step.is_some()
    }

    /// Render current transition frame into buffer. Returns true if transition is ongoing.
    pub fn render_frame(
        &mut self,
        width: usize,
        height: usize,
        buffer: &mut [u32],
    ) -> bool {
        if let Some(ref trans) = self.active_transition {
            let elapsed = trans.start_time.elapsed();
            let progress = (elapsed.as_secs_f32() / trans.duration.as_secs_f32()).clamp(0.0, 1.0);

            let mut ctx = RenderContext::new(width, height, buffer);
            trans
                .animation
                .render(&mut ctx, Some(&trans.from), &trans.to, progress);

            if progress >= 1.0 {
                self.active_transition = None;
                return false;
            }

            true
        } else {
            false
        }
    }

    /// Render active component animation overlay on top of buffer if any is running
    pub fn render_component_animation(
        &mut self,
        width: usize,
        height: usize,
        full_slide: &SlideSurface,
        buffer: &mut [u32],
        bg_color: u32,
    ) -> bool {
        if let Some(ref step) = self.active_component_step {
            let elapsed = step.start_time.elapsed();
            let progress = (elapsed.as_secs_f32() / step.duration.as_secs_f32()).clamp(0.0, 1.0);

            step.animation.render(
                progress, step.rect, full_slide, buffer, width, height, bg_color,
            );

            if progress >= 1.0 {
                self.active_component_step = None;
                return false;
            }
            true
        } else {
            false
        }
    }
}
