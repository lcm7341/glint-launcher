use weft::core::{Corners, Rect, RoundedRect};
use weft::gfx::{Canvas, LineCap, LineJoin, StrokeStyle};
use weft::prelude::{Color, Path, Response, Sense, Ui, Vec2, vec2};

use crate::ui::theme::{HOVERED, MUTED, RAISED, TEXT};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Star,
    StarFilled,
    More,
    Pin,
    PinFilled,
    Minimize,
    Maximize,
    Restore,
    Close,
}

const GRID: f32 = 20.0;
const STROKE: f32 = 1.6;

impl Icon {
    pub(crate) fn draw(self, canvas: &mut Canvas, origin: Vec2, size: f32, color: Color) {
        let scale = size / GRID;
        let point = |x: f32, y: f32| vec2(origin.x + x * scale, origin.y + y * scale);
        let stroke = StrokeStyle::new(STROKE * scale.max(1.0))
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Round);
        match self {
            Icon::Star | Icon::StarFilled => {
                let star = star_path(&point);
                if self == Icon::StarFilled {
                    canvas.fill_path(&star, color);
                } else {
                    canvas.stroke_path(&star, stroke, color);
                }
            }
            Icon::Pin | Icon::PinFilled => {
                let head = Path::builder()
                    .move_to(point(7.0, 3.5))
                    .line_to(point(13.0, 3.5))
                    .line_to(point(12.0, 8.5))
                    .line_to(point(14.5, 11.5))
                    .line_to(point(5.5, 11.5))
                    .line_to(point(8.0, 8.5))
                    .close()
                    .build();
                if self == Icon::PinFilled {
                    canvas.fill_path(&head, color);
                } else {
                    canvas.stroke_path(&head, stroke.clone(), color);
                }
                canvas.stroke_line(point(10.0, 11.5), point(10.0, 17.0), stroke, color);
            }
            Icon::More => {
                for x in [5.0, 10.0, 15.0] {
                    canvas.fill_circle(point(x, 10.0), 1.5 * scale, color);
                }
            }
            Icon::Minimize => {
                canvas.stroke_line(point(5.0, 10.0), point(15.0, 10.0), stroke, color);
            }
            Icon::Maximize => {
                let square = Rect::from_min_size(point(5.0, 5.0), vec2(10.0 * scale, 10.0 * scale));
                canvas.stroke(
                    RoundedRect::new(square, Corners::all(1.0 * scale)),
                    stroke,
                    color,
                );
            }
            Icon::Restore => {
                let front = Rect::from_min_size(point(5.0, 7.0), vec2(8.0 * scale, 8.0 * scale));
                canvas.stroke(
                    RoundedRect::new(front, Corners::all(1.0 * scale)),
                    stroke.clone(),
                    color,
                );
                let back = Path::builder()
                    .move_to(point(7.0, 7.0))
                    .line_to(point(7.0, 5.0))
                    .line_to(point(15.0, 5.0))
                    .line_to(point(15.0, 13.0))
                    .line_to(point(13.0, 13.0))
                    .build();
                canvas.stroke_path(&back, stroke, color);
            }
            Icon::Close => {
                canvas.stroke_line(point(5.5, 5.5), point(14.5, 14.5), stroke.clone(), color);
                canvas.stroke_line(point(14.5, 5.5), point(5.5, 14.5), stroke, color);
            }
        }
    }
}

fn star_path(point: &dyn Fn(f32, f32) -> Vec2) -> Path {
    let mut builder = Path::builder();
    for index in 0..10 {
        let radius = if index % 2 == 0 { 7.5 } else { 3.2 };
        let angle =
            std::f32::consts::PI * (f32::from(index as u8) / 5.0) - std::f32::consts::FRAC_PI_2;
        let at = point(10.0 + radius * angle.cos(), 10.5 + radius * angle.sin());
        if index == 0 {
            builder.move_to(at);
        } else {
            builder.line_to(at);
        }
    }
    builder.close().build()
}

pub(crate) const BUTTON: f32 = 36.0;
const ICON: f32 = 18.0;

pub(crate) fn icon_button(ui: &mut Ui, icon: Icon, name: &str, is_active: bool) -> Response {
    ui.canvas()
        .size(BUTTON, BUTTON)
        .sense(Sense::CLICK)
        .name(name)
        .show(|canvas, rect, response| {
            let fill = if is_active {
                Some(RAISED)
            } else if response.hovered() {
                Some(HOVERED)
            } else {
                None
            };
            if let Some(fill) = fill {
                let back = Rect::from_min_size(Vec2::ZERO, rect.size());
                canvas.fill_rounded_rect(RoundedRect::new(back, Corners::all(9.0)), fill);
            }
            let color = if is_active || response.hovered() {
                TEXT
            } else {
                MUTED
            };
            let inset = (rect.width() - ICON) / 2.0;
            icon.draw(canvas, vec2(inset, inset), ICON, color);
        })
}
