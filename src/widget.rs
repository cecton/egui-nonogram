//! The egui widget that renders and drives a [`NonogramGame`](crate::NonogramGame).

use egui::{
    emath::GuiRounding, Align2, Color32, CornerRadius, FontId, PointerButton, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, Visuals, Widget,
};

use crate::game::{CellState, GameStatus, NonogramGame};

/// Which action a plain (primary) tap performs. Desktop users always have
/// both actions available (left-click fills, right-click crosses); this
/// only matters for touch input, which has no secondary-tap gesture — a
/// toolbar toggle can flip it so mobile players can reach both actions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TapMode {
    /// A tap fills the cell.
    #[default]
    Fill,
    /// A tap crosses out the cell.
    Cross,
}

/// The mark a click-and-drag gesture is painting, decided once from the
/// cell the gesture started on and then applied to every cell the pointer
/// passes over for the rest of that gesture (stored in egui's per-widget
/// temporary memory so it survives across the frames of one drag).
#[derive(Clone, Copy, Debug, PartialEq)]
enum PaintTarget {
    Fill,
    Cross,
    Clear,
}

impl PaintTarget {
    fn apply(self, game: &mut NonogramGame, x: usize, y: usize) {
        match self {
            Self::Fill => game.fill(x, y),
            Self::Cross => game.cross(x, y),
            Self::Clear => game.clear_cell(x, y),
        }
    }
}

const CLUE_FONT_SIZE: f32 = 14.0;
const CLUE_CHAR_WIDTH: f32 = CLUE_FONT_SIZE * 0.6; // monospace approximation
const CLUE_LINE_HEIGHT: f32 = CLUE_FONT_SIZE * 1.3;
const GUTTER_PADDING: f32 = 8.0;
const PREVIEW_MAX_SIZE: f32 = 96.0;
const PREVIEW_GAP: f32 = 12.0;

/// An egui widget that renders a nonogram board with its row/column clues.
///
/// Left-click fills a cell; right-click crosses it out. Press and drag to
/// paint (or erase) a whole stroke of cells in one gesture — the action is
/// decided by the cell the drag started on and then repeated for every cell
/// the pointer passes over. Once a row or column's filled cells match its
/// clue, that clue is dimmed (and struck through) as a solving aid.
///
/// ```ignore
/// ui.add(egui_nonogram::NonogramWidget::new(&mut game));
/// ```
pub struct NonogramWidget<'a> {
    game: &'a mut NonogramGame,
    cell_size: Option<f32>,
    tap_mode: TapMode,
    show_preview: bool,
    win_message: Option<String>,
    interactive: bool,
}

impl<'a> NonogramWidget<'a> {
    pub fn new(game: &'a mut NonogramGame) -> Self {
        Self {
            game,
            cell_size: None,
            tap_mode: TapMode::Fill,
            show_preview: false,
            win_message: None,
            interactive: true,
        }
    }

    /// Override the size (in logical pixels) of each grid cell. When not
    /// set, the cell size is computed automatically to fill the available
    /// space of the parent container (after reserving room for the clue
    /// gutters).
    pub fn cell_size(mut self, size: f32) -> Self {
        self.cell_size = Some(size);
        self
    }

    /// What a plain tap/primary-click does. Defaults to [`TapMode::Fill`].
    pub fn tap_mode(mut self, mode: TapMode) -> Self {
        self.tap_mode = mode;
        self
    }

    /// Show a small read-only thumbnail of the current filled cells beside
    /// the board — a compact "how's it coming along" overview on larger
    /// grids. It only ever reflects cells the player has already filled, so
    /// it can't spoil a picture puzzle's solution. Defaults to `false`.
    pub fn show_preview(mut self, enabled: bool) -> Self {
        self.show_preview = enabled;
        self
    }

    /// Message shown in the win banner drawn over the board once
    /// [`GameStatus::Won`] is reached. Defaults to `"Solved!"` when not set.
    pub fn win_message(mut self, message: impl Into<String>) -> Self {
        self.win_message = Some(message.into());
        self
    }

    /// Whether the widget responds to taps/drags at all. Set to `false`
    /// to render the board read-only — e.g. while a surrounding
    /// container (like `egui::containers::Scene`) should own pointer
    /// gestures instead. Unlike wrapping the widget in a disabled `Ui`,
    /// this does not dim it: painting stays full-opacity, only the
    /// pointer sense changes. Defaults to `true`.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

fn clue_line_text(clue: &[u8]) -> String {
    clue.iter().map(u8::to_string).collect::<Vec<_>>().join(" ")
}

/// Row gutter width and column gutter height for a given game's clues.
fn gutter_size(game: &NonogramGame) -> Vec2 {
    let max_row_chars = game
        .row_clues
        .iter()
        .map(|c| clue_line_text(c).chars().count())
        .max()
        .unwrap_or(1);
    let max_col_lines = game.col_clues.iter().map(Vec::len).max().unwrap_or(1);

    let row_gutter_width = max_row_chars as f32 * CLUE_CHAR_WIDTH + GUTTER_PADDING;
    let col_gutter_height = max_col_lines as f32 * CLUE_LINE_HEIGHT + GUTTER_PADDING * 0.5;
    Vec2::new(row_gutter_width, col_gutter_height)
}

/// The total footprint [`NonogramWidget`] will occupy for `game` at a given
/// `cell_size`, including the clue gutters and (if enabled) the progress
/// preview column. Lets a caller pre-size a container (e.g. `egui::Scene`)
/// before laying the widget out.
pub fn content_size(game: &NonogramGame, cell_size: f32, show_preview: bool) -> Vec2 {
    let board_size = Vec2::new(game.width as f32, game.height as f32) * cell_size;
    let preview_width = if show_preview {
        PREVIEW_MAX_SIZE + PREVIEW_GAP
    } else {
        0.0
    };
    board_size + gutter_size(game) + Vec2::new(preview_width, 0.0)
}

impl Widget for NonogramWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let game = self.game;
        let font_id = FontId::monospace(CLUE_FONT_SIZE);

        let row_texts: Vec<String> = game.row_clues.iter().map(|c| clue_line_text(c)).collect();
        let gutter_size = gutter_size(game);
        let row_gutter_width = gutter_size.x;
        let col_gutter_height = gutter_size.y;
        let preview_width = if self.show_preview {
            PREVIEW_MAX_SIZE + PREVIEW_GAP
        } else {
            0.0
        };

        let cell_size = self.cell_size.unwrap_or_else(|| {
            let available = ui.available_size();
            let by_width = (available.x - row_gutter_width - preview_width) / game.width as f32;
            let by_height = (available.y - col_gutter_height) / game.height as f32;
            by_width.min(by_height).max(4.0)
        });

        let board_size = Vec2::new(game.width as f32, game.height as f32) * cell_size;
        let total_size = board_size + gutter_size + Vec2::new(preview_width, 0.0);

        let sense = if self.interactive {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (response, painter) = ui.allocate_painter(total_size, sense);
        let origin = response.rect.min + gutter_size;
        let width = game.width;
        let height = game.height;
        let cell_at = |pos: Pos2| -> Option<(usize, usize)> {
            let local = pos - origin;
            if local.x >= 0.0 && local.y >= 0.0 && local.x < board_size.x && local.y < board_size.y
            {
                let cx = (local.x / cell_size).floor() as usize;
                let cy = (local.y / cell_size).floor() as usize;
                if cx < width && cy < height {
                    return Some((cx, cy));
                }
            }
            None
        };

        // ── Input handling ──────────────────────────────────────────────
        // A gesture (plain click or the start of a drag) decides its paint
        // target from the cell it started on, stores that decision in
        // egui's per-widget temp memory, and re-applies it (idempotently)
        // to every cell touched for the rest of the gesture — so dragging
        // back over an already-painted cell doesn't undo it.
        let paint_id = response.id.with("paint_target");
        let gesture_start = response.clicked()
            || response.secondary_clicked()
            || response.drag_started_by(PointerButton::Primary)
            || response.drag_started_by(PointerButton::Secondary);
        if gesture_start {
            if let Some((cx, cy)) = response.interact_pointer_pos().and_then(cell_at) {
                let secondary = response.secondary_clicked()
                    || response.drag_started_by(PointerButton::Secondary);
                let current = game.cells[game.idx(cx, cy)];
                let target = if secondary {
                    if current == CellState::Crossed {
                        PaintTarget::Clear
                    } else {
                        PaintTarget::Cross
                    }
                } else {
                    match self.tap_mode {
                        TapMode::Fill if current == CellState::Filled => PaintTarget::Clear,
                        TapMode::Fill => PaintTarget::Fill,
                        TapMode::Cross if current == CellState::Crossed => PaintTarget::Clear,
                        TapMode::Cross => PaintTarget::Cross,
                    }
                };
                ui.ctx().data_mut(|d| d.insert_temp(paint_id, target));
                target.apply(game, cx, cy);
            }
        } else if response.dragged() {
            if let Some((cx, cy)) = response.interact_pointer_pos().and_then(cell_at) {
                let target = ui.ctx().data(|d| d.get_temp::<PaintTarget>(paint_id));
                if let Some(target) = target {
                    target.apply(game, cx, cy);
                }
            }
        }

        // ── Painting ─────────────────────────────────────────────────────
        let visuals = ui.visuals();
        let normal_color = visuals.text_color();
        // `visuals.weak_text_color()` is too close to `text_color()` to read
        // as "satisfied" at a glance against this widget's dark cell
        // background, so dim explicitly and strongly instead.
        let dim_color = normal_color.gamma_multiply(0.35);

        // Row clues, right-aligned in the left gutter.
        for (y, row_text) in row_texts.iter().enumerate() {
            let satisfied = game.row_satisfied(y);
            let color = if satisfied { dim_color } else { normal_color };
            let pos = Pos2::new(
                response.rect.min.x + row_gutter_width - GUTTER_PADDING * 0.5,
                origin.y + (y as f32 + 0.5) * cell_size,
            );
            let text_rect =
                painter.text(pos, Align2::RIGHT_CENTER, row_text, font_id.clone(), color);
            if satisfied {
                let mid_y = text_rect.center().y;
                painter.line_segment(
                    [
                        Pos2::new(text_rect.min.x, mid_y),
                        Pos2::new(text_rect.max.x, mid_y),
                    ],
                    Stroke::new(1.5, color),
                );
            }
        }

        // Column clues, stacked bottom-up in the top gutter. Satisfied
        // columns get a vertical strikethrough spanning the whole stack —
        // the rotated equivalent of a row's horizontal strikethrough, since
        // a single horizontal line can't sensibly cross several stacked
        // numbers.
        for x in 0..game.width {
            let satisfied = game.col_satisfied(x);
            let color = if satisfied { dim_color } else { normal_color };
            let x_center = origin.x + (x as f32 + 0.5) * cell_size;
            let mut stack_top = f32::MAX;
            let mut stack_bottom = f32::MIN;
            for (i, n) in game.col_clues[x].iter().rev().enumerate() {
                let y_center = origin.y - (i as f32 + 0.5) * CLUE_LINE_HEIGHT;
                let text_rect = painter.text(
                    Pos2::new(x_center, y_center),
                    Align2::CENTER_CENTER,
                    n.to_string(),
                    font_id.clone(),
                    color,
                );
                stack_top = stack_top.min(text_rect.min.y);
                stack_bottom = stack_bottom.max(text_rect.max.y);
            }
            if satisfied && stack_top < stack_bottom {
                painter.line_segment(
                    [
                        Pos2::new(x_center, stack_top),
                        Pos2::new(x_center, stack_bottom),
                    ],
                    Stroke::new(1.5, color),
                );
            }
        }

        // Grid cells.
        for y in 0..game.height {
            for x in 0..game.width {
                let cell_rect = Rect::from_min_size(
                    origin + Vec2::new(x as f32, y as f32) * cell_size,
                    Vec2::splat(cell_size),
                );
                draw_cell(&painter, cell_rect, game.cells[game.idx(x, y)], visuals);
            }
        }

        // Major gridlines: painted on top of the cells so they read
        // clearly regardless of Empty/Filled/Crossed state underneath.
        draw_major_gridlines(&painter, origin, board_size, width, height, cell_size, normal_color);

        // Progress preview: a compact, read-only thumbnail of the filled
        // cells only, to the right of the board.
        if self.show_preview {
            let preview_cell = PREVIEW_MAX_SIZE / game.width.max(game.height) as f32;
            let preview_origin =
                Pos2::new(response.rect.max.x - PREVIEW_MAX_SIZE, response.rect.min.y);
            let preview_size = Vec2::new(game.width as f32, game.height as f32) * preview_cell;
            painter.rect_filled(
                Rect::from_min_size(preview_origin, preview_size),
                CornerRadius::ZERO,
                visuals.extreme_bg_color,
            );
            for y in 0..game.height {
                for x in 0..game.width {
                    if game.cells[game.idx(x, y)] == CellState::Filled {
                        let cell_rect = Rect::from_min_size(
                            preview_origin + Vec2::new(x as f32, y as f32) * preview_cell,
                            Vec2::splat(preview_cell),
                        );
                        painter.rect_filled(cell_rect, CornerRadius::ZERO, normal_color);
                    }
                }
            }
            painter.rect_stroke(
                Rect::from_min_size(preview_origin, preview_size),
                CornerRadius::ZERO,
                Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
                StrokeKind::Outside,
            );
        }

        // Win banner: drawn last so it sits on top, sized to the board
        // itself (excludes the clue gutters and the preview column) so it
        // never covers the picture preview.
        if game.status == GameStatus::Won {
            let board_rect = Rect::from_min_size(origin, board_size);
            let banner_size = Vec2::new(
                (board_size.x - 20.0).max(20.0),
                64.0_f32.min(board_size.y - 4.0).max(20.0),
            );
            let banner = Rect::from_center_size(board_rect.center(), banner_size);
            painter.rect_filled(banner, 8.0, Color32::from_black_alpha(170));
            painter.text(
                banner.center(),
                Align2::CENTER_CENTER,
                self.win_message.as_deref().unwrap_or("Solved!"),
                TextStyle::Heading.resolve(ui.style()),
                Color32::from_rgb(170, 240, 170),
            );
        }

        response
    }
}

fn draw_cell(painter: &egui::Painter, rect: Rect, state: CellState, visuals: &Visuals) {
    let ppi = painter.ctx().pixels_per_point();
    let rect = rect.round_to_pixels(ppi);
    let inner = rect.shrink(1.0);

    match state {
        CellState::Empty => {
            painter.rect_filled(inner, CornerRadius::ZERO, visuals.extreme_bg_color);
        }
        CellState::Filled => {
            let fill = if visuals.dark_mode {
                Color32::from_rgb(225, 225, 230)
            } else {
                Color32::from_rgb(35, 35, 40)
            };
            painter.rect_filled(inner, CornerRadius::ZERO, fill);
        }
        CellState::Crossed => {
            painter.rect_filled(inner, CornerRadius::ZERO, visuals.extreme_bg_color);
            let stroke_color = visuals.widgets.noninteractive.fg_stroke.color;
            let pad = inner.width() * 0.28;
            painter.line_segment(
                [inner.min + Vec2::splat(pad), inner.max - Vec2::splat(pad)],
                Stroke::new(2.0, stroke_color),
            );
            painter.line_segment(
                [
                    Pos2::new(inner.min.x + pad, inner.max.y - pad),
                    Pos2::new(inner.max.x - pad, inner.min.y + pad),
                ],
                Stroke::new(2.0, stroke_color),
            );
        }
    }

    painter.rect_stroke(
        inner,
        CornerRadius::ZERO,
        Stroke::new(0.5, visuals.widgets.noninteractive.bg_stroke.color),
        StrokeKind::Inside,
    );
}

// Bold guide lines every 5th internal cell boundary — the standard
// nonogram/sudoku convention for helping players count large grids.
// Only internal boundaries are drawn (the board's outer edge keeps its
// plain per-cell border from `draw_cell`); a board with 5 or fewer
// rows/columns in a dimension naturally gets none, with no special-casing.
fn draw_major_gridlines(
    painter: &egui::Painter,
    origin: Pos2,
    board_size: Vec2,
    width: usize,
    height: usize,
    cell_size: f32,
    color: Color32,
) {
    let ppi = painter.ctx().pixels_per_point();
    let stroke = Stroke::new(1.5, color);

    let mut n = 5;
    while n < width {
        let x = origin.x + n as f32 * cell_size;
        painter.line_segment(
            [
                Pos2::new(x, origin.y).round_to_pixels(ppi),
                Pos2::new(x, origin.y + board_size.y).round_to_pixels(ppi),
            ],
            stroke,
        );
        n += 5;
    }

    let mut n = 5;
    while n < height {
        let y = origin.y + n as f32 * cell_size;
        painter.line_segment(
            [
                Pos2::new(origin.x, y).round_to_pixels(ppi),
                Pos2::new(origin.x + board_size.x, y).round_to_pixels(ppi),
            ],
            stroke,
        );
        n += 5;
    }
}
