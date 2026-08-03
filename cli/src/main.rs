//! Terminal hotseat playground for validating `dab-core`.
//!
//! ```text
//! cargo run -p dab-cli
//! cargo run -p dab-cli -- --rows 2 --cols 2
//! ```

mod render;

use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use dab_core::{
    BoardGeom, EdgeCoord, EdgeId, Game, MoveError, Orientation, Player, Winner, MAX_COLS, MAX_ROWS,
};

use render::{player_label, render_board, render_help, render_legal, render_status};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return ExitCode::SUCCESS;
    }

    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let args = Args::parse(args)?;
    let geom = BoardGeom::new(args.rows, args.cols).ok_or_else(|| {
        format!(
            "invalid board size {}x{} (supported: 1..={MAX_ROWS} by 1..={MAX_COLS})",
            args.rows, args.cols
        )
    })?;

    let mut game = Game::new(geom);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    writeln!(
        stdout,
        "Dots and Boxes — hotseat CLI (dab-core playground)\nBoard: {}×{} boxes\n",
        geom.rows(),
        geom.cols()
    )
    .map_err(io_err)?;
    writeln!(stdout, "{}", render_help(geom)).map_err(io_err)?;
    print_state(&mut stdout, &game)?;

    loop {
        if game.is_terminal() {
            print_terminal(&mut stdout, &game)?;
            break;
        }

        write!(stdout, "[{}] move> ", player_label(game.current_player())).map_err(io_err)?;
        stdout.flush().map_err(io_err)?;

        let Some(line) = lines.next() else {
            writeln!(stdout).map_err(io_err)?;
            break;
        };
        let line = line.map_err(io_err)?;
        let command = match Command::parse(&line, geom) {
            Ok(cmd) => cmd,
            Err(msg) => {
                writeln!(stdout, "{msg}  (type 'help')").map_err(io_err)?;
                continue;
            }
        };

        match command {
            Command::Quit => {
                writeln!(stdout, "Bye.").map_err(io_err)?;
                break;
            }
            Command::Help => {
                writeln!(stdout, "{}", render_help(geom)).map_err(io_err)?;
            }
            Command::Legal => {
                writeln!(stdout, "{}", render_legal(&game)).map_err(io_err)?;
            }
            Command::Board => {
                print_state(&mut stdout, &game)?;
            }
            Command::Play(edge) => match play_edge(&mut game, edge) {
                Ok(msg) => {
                    writeln!(stdout, "{msg}").map_err(io_err)?;
                    print_state(&mut stdout, &game)?;
                }
                Err(msg) => writeln!(stdout, "{msg}").map_err(io_err)?,
            },
        }
    }

    Ok(())
}

fn play_edge(game: &mut Game, edge: EdgeId) -> Result<String, String> {
    let mover = game.current_player();
    let (result, _) = game.play(edge).map_err(|e| match e {
        MoveError::AlreadyDrawn => format!("edge #{edge} is already drawn"),
        MoveError::OutOfRange => format!("edge #{edge} is out of range"),
    })?;

    let mut msg = format!("{} played #{}", player_label(mover), edge);
    if !result.completed.is_empty() {
        msg.push_str(&format!(
            " — claimed {} box(es), extra turn!",
            result.completed.len()
        ));
    }
    Ok(msg)
}

fn print_state(stdout: &mut impl Write, game: &Game) -> Result<(), String> {
    writeln!(stdout).map_err(io_err)?;
    writeln!(stdout, "{}", render_board(game)).map_err(io_err)?;
    writeln!(stdout, "{}", render_status(game)).map_err(io_err)?;
    writeln!(stdout).map_err(io_err)?;
    Ok(())
}

fn print_terminal(stdout: &mut impl Write, game: &Game) -> Result<(), String> {
    writeln!(stdout, "\n=== Game over ===").map_err(io_err)?;
    writeln!(stdout, "{}", render_status(game)).map_err(io_err)?;
    match game.winner() {
        Some(Winner::Player(Player::P1)) => writeln!(stdout, "Winner: P1").map_err(io_err)?,
        Some(Winner::Player(Player::P2)) => writeln!(stdout, "Winner: P2").map_err(io_err)?,
        Some(Winner::Draw) => writeln!(stdout, "Draw!").map_err(io_err)?,
        None => writeln!(stdout, "(no winner — unexpected)").map_err(io_err)?,
    }
    Ok(())
}

fn io_err(err: io::Error) -> String {
    err.to_string()
}

#[derive(Debug)]
struct Args {
    rows: u8,
    cols: u8,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut rows = 2u8;
        let mut cols = 2u8;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--rows" => {
                    rows = parse_u8(
                        &iter
                            .next()
                            .ok_or_else(|| "--rows needs a value".to_string())?,
                        "--rows",
                    )?;
                }
                "--cols" => {
                    cols = parse_u8(
                        &iter
                            .next()
                            .ok_or_else(|| "--cols needs a value".to_string())?,
                        "--cols",
                    )?;
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown flag '{other}' (try --help)"));
                }
                other => return Err(format!("unexpected argument '{other}' (try --help)")),
            }
        }
        Ok(Self { rows, cols })
    }
}

fn parse_u8(raw: &str, flag: &str) -> Result<u8, String> {
    raw.parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer, got '{raw}'"))
}

fn print_usage() {
    eprintln!(
        "\
Usage: dab-cli [--rows N] [--cols M]

Hotseat Dots and Boxes playground backed by dab-core.

Options:
  --rows N   box rows (default 2, max {MAX_ROWS})
  --cols M   box cols (default 2, max {MAX_COLS})
  -h, --help show this help
"
    );
}

#[derive(Debug)]
enum Command {
    Play(EdgeId),
    Legal,
    Board,
    Help,
    Quit,
}

impl Command {
    fn parse(line: &str, geom: BoardGeom) -> Result<Self, String> {
        let line = line.trim();
        if line.is_empty() {
            return Err("empty input".into());
        }

        let lower = line.to_ascii_lowercase();
        match lower.as_str() {
            "q" | "quit" | "exit" => return Ok(Self::Quit),
            "help" | "?" => return Ok(Self::Help),
            "legal" | "moves" | "l" => return Ok(Self::Legal),
            "board" | "b" => return Ok(Self::Board),
            _ => {}
        }

        let mut parts = line.split_whitespace();
        let head = parts.next().unwrap();
        let head_upper = head.to_ascii_uppercase();

        if head_upper == "H" || head_upper == "V" {
            let row: u8 = parts
                .next()
                .ok_or_else(|| "expected: H <row> <col>  or  V <row> <col>".to_string())?
                .parse()
                .map_err(|_| "row must be an integer".to_string())?;
            let col: u8 = parts
                .next()
                .ok_or_else(|| "expected: H <row> <col>  or  V <row> <col>".to_string())?
                .parse()
                .map_err(|_| "col must be an integer".to_string())?;
            if parts.next().is_some() {
                return Err("too many arguments".into());
            }
            let orientation = if head_upper == "H" {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            };
            let id = geom
                .edge_id(EdgeCoord {
                    orientation,
                    row,
                    col,
                })
                .ok_or_else(|| {
                    format!("{head_upper} {row} {col} is out of range for this board")
                })?;
            return Ok(Self::Play(id));
        }

        let id_str = head.trim_start_matches('#');
        if parts.next().is_some() {
            return Err("too many arguments (use: <id>  or  H <row> <col>)".into());
        }
        let id: EdgeId = id_str
            .parse()
            .map_err(|_| format!("unknown command '{line}' (try 'help')"))?;
        Ok(Self::Play(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coord_and_id() {
        let geom = BoardGeom::new(2, 2).unwrap();
        match Command::parse("H 0 1", geom).unwrap() {
            Command::Play(id) => {
                let c = geom.edge_coord(id).unwrap();
                assert_eq!(c.orientation, Orientation::Horizontal);
                assert_eq!((c.row, c.col), (0, 1));
            }
            other => panic!("unexpected {other:?}"),
        }
        match Command::parse("#3", geom).unwrap() {
            Command::Play(3) => {}
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            Command::parse("legal", geom).unwrap(),
            Command::Legal
        ));
        assert!(matches!(Command::parse("q", geom).unwrap(), Command::Quit));
    }

    #[test]
    fn args_defaults_and_flags() {
        let a = Args::parse(vec![]).unwrap();
        assert_eq!((a.rows, a.cols), (2, 2));
        let a = Args::parse(vec![
            "--rows".into(),
            "3".into(),
            "--cols".into(),
            "4".into(),
        ])
        .unwrap();
        assert_eq!((a.rows, a.cols), (3, 4));
    }
}
