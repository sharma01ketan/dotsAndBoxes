//! ASCII rendering of a [`dab_core::Game`] for the terminal playground.

use dab_core::{BoardGeom, EdgeCoord, EdgeId, Game, Orientation, Player};

pub fn player_label(player: Player) -> &'static str {
    match player {
        Player::P1 => "P1",
        Player::P2 => "P2",
    }
}

/// Draw the board: dots, drawn/empty edges, and claimed box owners.
pub fn render_board(game: &Game) -> String {
    let geom = game.geom();
    let rows = geom.rows() as usize;
    let cols = geom.cols() as usize;
    let pos = game.position();
    let mut out = String::new();

    out.push_str("    ");
    for c in 0..cols {
        out.push_str(&format!(" {c}  "));
    }
    out.push('\n');

    for r in 0..=rows {
        out.push_str(&format!("{r:>2}  "));
        for c in 0..cols {
            out.push('*');
            let id = geom
                .edge_id(EdgeCoord {
                    orientation: Orientation::Horizontal,
                    row: r as u8,
                    col: c as u8,
                })
                .expect("horizontal edge in range");
            if pos.edge_is_drawn(id) {
                out.push_str("---");
            } else {
                out.push_str(" · ");
            }
        }
        out.push('*');
        out.push('\n');

        if r == rows {
            break;
        }

        out.push_str("    ");
        for c in 0..=cols {
            let id = geom
                .edge_id(EdgeCoord {
                    orientation: Orientation::Vertical,
                    row: r as u8,
                    col: c as u8,
                })
                .expect("vertical edge in range");
            if pos.edge_is_drawn(id) {
                out.push('|');
            } else {
                out.push('·');
            }

            if c < cols {
                let box_id = geom.box_id(r as u8, c as u8).expect("box in range");
                let cell = match game.box_owner(box_id) {
                    Some(Player::P1) => " 1 ",
                    Some(Player::P2) => " 2 ",
                    None => "   ",
                };
                out.push_str(cell);
            }
        }
        out.push('\n');
    }

    out
}

pub fn render_status(game: &Game) -> String {
    format!(
        "Score  P1={}  P2={}    Turn: {}",
        game.score(Player::P1),
        game.score(Player::P2),
        player_label(game.current_player())
    )
}

pub fn format_edge(geom: BoardGeom, id: EdgeId) -> String {
    match geom.edge_coord(id) {
        Some(EdgeCoord {
            orientation: Orientation::Horizontal,
            row,
            col,
        }) => format!("#{id}  H {row} {col}"),
        Some(EdgeCoord {
            orientation: Orientation::Vertical,
            row,
            col,
        }) => format!("#{id}  V {row} {col}"),
        None => format!("#{id}  (invalid)"),
    }
}

pub fn render_legal(game: &Game) -> String {
    let geom = game.geom();
    let mut lines: Vec<String> = game.legal_moves().map(|id| format_edge(geom, id)).collect();
    lines.sort();
    if lines.is_empty() {
        "No legal moves.".to_string()
    } else {
        format!("Legal moves ({}):\n  {}", lines.len(), lines.join("\n  "))
    }
}

pub fn render_help(geom: BoardGeom) -> String {
    format!(
        "\
Commands:
  H <row> <col>   draw horizontal edge  (row 0..={rows}, col 0..{cols_ex})
  V <row> <col>   draw vertical edge    (row 0..{rows_ex}, col 0..={cols})
  <id>            draw edge by dense id (0..{edge_max})
  legal           list undrawn edges
  board           redraw the board
  help            show this help
  quit / q        exit

Board key:  --- / |  drawn edges    ·  undrawn    1/2  claimed by P1/P2
",
        rows = geom.rows(),
        cols = geom.cols(),
        rows_ex = geom.rows().saturating_sub(1),
        cols_ex = geom.cols().saturating_sub(1),
        edge_max = geom.edge_count().saturating_sub(1),
    )
}
