use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use base62;
use mysql_async::prelude::*;
use sonyflake::Sonyflake;
use warp::http::Uri;
use warp::Filter;

fn full_short_url(short_code: &str) -> String {
    let default_domain = "http://localhost:3030/";
    let base_domain = env::var("SHORTEN_DOMAIN").unwrap_or(default_domain.to_string());
    format!("{}{}", base_domain, short_code)
}

async fn shorten(form_data: HashMap<String, String>) -> Result<impl warp::Reply, warp::Rejection> {
    let url = form_data
        .get("url")
        .unwrap_or(&String::from(""))
        .to_string();

    let database_url =
        mysql_async::Opts::from_url("mysql://user:password@127.0.0.1:3306/shorten").unwrap();
    let pool = mysql_async::Pool::new(database_url);
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");

    let short_codes = conn
        .exec_map(
            r"select short_code from url_mapping where original_url = :original_url",
            params! {
                "original_url" => &url,
            },
            |short_code: String| short_code,
        )
        .await
        .expect("Failed to select query.");

    if short_codes.len() > 0 {
        let short_url = full_short_url(&short_codes[0]);
        println!("Found short URL: {}", short_url);
        return Ok(warp::reply::html(format!(
            r#"
            <p>Shortened URL: <a href="{short_url}">{short_url}</a></p>
            "#,
            short_url = short_url,
        )));
    }

    let flake = Sonyflake::new().unwrap();
    let id = flake.next_id().unwrap();
    let short_code = base62::encode(id);
    println!("id: {}, Generated short URL: {}", id, short_code);

    conn.exec_drop(
        r"INSERT INTO url_mapping (id, short_code, original_url) VALUES (:id, :short_code, :original_url)",
        params! {
            "id" => &id,
            "short_code" => &short_code,
            "original_url" => &url,
        },
    )
    .await
    .expect("Failed to insert data.");
    drop(conn);

    let short_url = full_short_url(&short_code);
    Ok(warp::reply::html(format!(
        r#"
        <p>Shortened URL: <a href="{short_url}">{short_url}</a></p>
        "#,
        short_url = short_url,
    )))
}

async fn redirect(short: String) -> Result<impl warp::Reply, warp::Rejection> {
    let database_url =
        mysql_async::Opts::from_url("mysql://user:password@127.0.0.1:3306/shorten").unwrap();
    let pool = mysql_async::Pool::new(database_url);
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");

    let urls = conn
        .exec_map(
            r"select original_url from url_mapping where short_code = :short_code",
            params! {
                "short_code" => &short,
            },
            |original_url: String| original_url,
        )
        .await
        .expect("Failed to select query.");

    if urls.len() > 0 {
        println!("Found original URL: {}", urls[0]);
        let target_uri = Uri::from_str(&urls[0]).unwrap();
        println!("Redirect to: {}", target_uri);
        return Ok(warp::redirect(target_uri));
    }

    println!("Not found original URL");
    Err(warp::reject::not_found())
}

#[tokio::main]
async fn main() {
    let index = warp::path::end().map(|| {
        warp::reply::html(
            r#"
            <form action="/shorten" method="post">
                <input type="text" name="url">
                <input type="submit" value="Shorten">
            </form>
            "#,
        )
    });

    let shorten_route = warp::path!("shorten")
        .and(warp::post())
        .and(warp::body::form())
        .and_then(shorten);
    let redirect_route = warp::path!(String).and_then(redirect);
    let routes = index.or(shorten_route).or(redirect_route);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
