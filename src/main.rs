use std::str::FromStr;

use base62;
use mysql_async::prelude::*;
use sonyflake::Sonyflake;
use warp::http::Uri;
use warp::Filter;

async fn shorten(body: warp::hyper::body::Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let url = String::from_utf8(body.to_vec()).unwrap_or_default();

    let database_url =
        mysql_async::Opts::from_url("mysql://user:password@127.0.0.1:3306/shorten").unwrap();
    let pool = mysql_async::Pool::new(database_url);
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");

    let short_urls = conn
        .exec_map(
            r"select short_url from url_mapping where original_url = :original_url",
            params! {
                "original_url" => &url,
            },
            |short_url: String| short_url,
        )
        .await
        .expect("Failed to select data.");

    if short_urls.len() > 0 {
        println!("Found short URL: {}", short_urls[0]);
        return Ok(warp::reply::html(format!(
            r#"
            <p>Shortened URL: <a href="{short_url}">{short_url}</a></p>
            "#,
            short_url = short_urls[0],
        )));
    }

    let flake = Sonyflake::new().unwrap();
    let id = flake.next_id().unwrap();
    let short_url = base62::encode(id);
    println!("id: {}, Generated short URL: {}", id, short_url);

    conn.exec_drop(
        r"INSERT INTO url_mapping (id, short_url, original_url) VALUES (:id, :short_url, :original_url)",
        params! {
            "id" => &id,
            "short_url" => &short_url,
            "original_url" => &url,
        },
    )
    .await
    .expect("Failed to insert data.");
    drop(conn);

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
            r"select original_url from url_mapping where short_url = :short_url",
            params! {
                "short_url" => &short,
            },
            |original_url: String| original_url,
        )
        .await
        .expect("Failed to select data.");

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
        .and(warp::body::bytes())
        .and_then(shorten);

    let redirect_route = warp::path!("u" / String).and_then(redirect);

    let routes = index.or(shorten_route).or(redirect_route);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
