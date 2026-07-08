//! StepDex — Walk-to-Earn Pokémon rewards, computed at the Fastly edge.
//!
//! Routes:
//!   GET /                -> the neon dashboard (leaderboard JSON injected inline)
//!   GET /api/leaderboard -> the computed leaderboard as JSON
//!   GET /api/steps.csv   -> the raw steps CSV (download)

mod model;
mod ui;

use fastly::http::{header, Method, StatusCode};
use fastly::{mime, Error, Request, Response};

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    println!(
        "FASTLY_SERVICE_VERSION: {}",
        std::env::var("FASTLY_SERVICE_VERSION").unwrap_or_else(|_| String::new())
    );

    // Only read methods are supported.
    match req.get_method() {
        &Method::GET | &Method::HEAD => {}
        _ => {
            return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET, HEAD")
                .with_body_text_plain("This method is not allowed\n"));
        }
    }

    match req.get_path() {
        "/" => {
            let board = model::compute();
            let json = serde_json::to_string(&board).unwrap_or_else(|_| "null".to_string());
            Ok(Response::from_status(StatusCode::OK)
                .with_content_type(mime::TEXT_HTML_UTF_8)
                .with_body(ui::render(&json)))
        }

        "/api/leaderboard" => {
            let board = model::compute();
            let json = serde_json::to_string_pretty(&board)?;
            Ok(Response::from_status(StatusCode::OK)
                .with_content_type(mime::APPLICATION_JSON)
                .with_body(json))
        }

        "/api/steps.csv" => Ok(Response::from_status(StatusCode::OK)
            .with_content_type(mime::TEXT_CSV_UTF_8)
            .with_header(
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"steps.csv\"",
            )
            .with_body(model::steps_csv())),

        _ => Ok(Response::from_status(StatusCode::NOT_FOUND).with_body_text_plain("Not found\n")),
    }
}
