use std::{convert::Infallible, env};
use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use chrono::Utc;
use serde_derive::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use warp::{Filter, Reply, filters::BoxedFilter, hyper::Uri};

#[derive(Serialize, Deserialize, Debug)]
pub struct Leaderboard {
    total_count: Option<i64>,
    items: Vec<Rank>
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Rank {
    username: Option<String>,
    rank: Option<i64>,
    points: Option<i64>
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL")?).await?;
        
    let founder_leaderboard = warp::path!("leaderboard")
    .and(warp::query::<HashMap<String, String>>())
    .and(with_db(db.clone()))
    .and_then(handle_leaderboard);

    let founder_points = warp::path!("email" / String / "founder_points")
    .and(with_db(db.clone()))
    .and_then(handle_founder_points);

    let answer = warp::path!("submit" / "answer")
    .and(warp::query::<HashMap<String, String>>())
    .and(with_db(db.clone()))
    .and_then(handle_submit_answer);

    let health = warp::path::end().map(|| warp::reply());

    let routes = 
        health
        .or(founder_leaderboard)
        .or(founder_points)
        .or(answer)
        .or(assets_filter());

    let port = env::var("PORT")?.parse::<u16>()?;
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;

    Ok(())
}
pub fn assets_filter() -> BoxedFilter<(impl Reply,)> {
    warp::path("assets").and(warp::fs::dir("./assets")).boxed()
}
fn with_db(db: Pool<Postgres>) -> impl Filter<Extract = (Pool<Postgres>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

pub async fn handle_leaderboard(params: HashMap<String, String>, db: Pool<Postgres>) -> Result<impl warp::Reply, Infallible> {
    let mut count = 10;
    let mut skip = 0;
    if let Some(str_value) = params.get("count") { 
        if let Ok(int_value) = str_value.parse() {
            if int_value >= 0 {
                count = int_value;
            }
        }
    }
    if let Some(str_value) = params.get("skip") { 
        if let Ok(int_value) = str_value.parse() {
            if int_value >= 0 {
                skip = int_value;
            }
        }
    }

    let total_count = match sqlx::query!("SELECT COUNT(DISTINCT(email)) as total_count FROM founder_answers")
    .fetch_one(&db)
    .await {
        Ok(res) => res.total_count,
        Err(e) => {
            println!("{:?}", e);
            Some(0)
        }
    };

    let leaderboard = match sqlx::query_as!(Rank, r#"
        SELECT points, RANK() OVER (ORDER BY points DESC) rank, username 
        FROM (
            SELECT email, MAX(username) as username,  COUNT(DISTINCT(question)) * 10 as points 
            FROM founder_answers 
            GROUP BY email
        ) 
        LIMIT $1 OFFSET $2"#, 
        count, 
        skip
    )
    .fetch_all(&db)
    .await {
        Ok(items) => Leaderboard{total_count: total_count, items: items},
        Err(e) => {
            println!("{:?}", e);
            Leaderboard{total_count: Some(0), items: vec![]}
        }
    };
    println!("{} \n- handler: {} \n- req: {:?} \n- res: {:?}", Utc::now(), "handle_leaderboard", &params, &leaderboard);
    return Ok(warp::reply::with_header(warp::reply::json(&leaderboard), "Access-Control-Allow-Origin", "*"));
}

pub async fn handle_founder_points(email: String, db: Pool<Postgres>) -> Result<impl warp::Reply, Infallible> {
    let total_count = match sqlx::query!("SELECT COUNT(DISTINCT(email)) as total_count FROM founder_answers")
    .fetch_one(&db)
    .await {
        Ok(res) => res.total_count,
        Err(e) => {
            println!("{:?}", e);
            Some(0)
        }
    };
    let rank = match sqlx::query_as!(Rank, r#"
        SELECT username, points, RANK() OVER (ORDER BY points) rank 
        FROM (
            SELECT email, MAX(username) as username, COUNT(DISTINCT(question)) * 10 as points 
            FROM founder_answers GROUP BY email
        ) 
        WHERE email = $1"#, 
        email
    )
    .fetch_one(&db)
    .await {
        Ok(info) => info,
        Err(e) => {
            println!("{:?}", e);
            Rank{username: Some(String::from("")), points:Some(0), rank:Some(0)}
        }
    };
    let founder_points = Leaderboard{total_count: total_count, items: vec![rank]};
    println!("{} \n- handler: {} \n- req: {:?} \n- res: {:?}", Utc::now(), "handle_founder_points", &email, &founder_points);
    Ok(warp::reply::with_header(warp::reply::json(&founder_points), "Access-Control-Allow-Origin", "*"))
}

pub async fn handle_submit_answer(form: HashMap<String, String>, db: Pool<Postgres>) -> Result<impl warp::Reply, Infallible> {
    let mut redirect_uri = Uri::default();
    let fail_url = if let Some(url) = form.get("fail_url") { url } else {""};
    if let Ok(uri) = Uri::from_str(&fail_url) {
        redirect_uri = uri;
    }
    if let Some(email) = form.get("from_email") {
        let username = if let Some(username) = form.get("from_username") { username } else { "" };
        store_answers(db, email, username, form.clone()).await;

        //redirect to leaderboard
        if let Some(url) = form.get("success_url") {
            if let Ok(uri) = Uri::from_str(&format!("{}?from_email={}&from_username={}", url, email, username)) {
                redirect_uri = uri;
            }
        }
    }
    println!("{} \n- handler: {} \n- req: {:?} \n- res: {:?}", Utc::now(), "handle_submit_answer", &form, &redirect_uri);
    return Ok(warp::redirect::temporary(redirect_uri));
}
pub async fn store_answers(db: Pool<Postgres>, email: &str, username: &str, form: HashMap<String, String>) {
    for (key, value) in form {
        if !value.is_empty() {
            match key.as_str() {
                "favorite_creative_sandbox" |
                "gaming_hours_past_week" |
                "minecraft_hours" |
                "roblox_hours" |
                "terraria_hours" |
                "garrysmod_hours" |
                "gaming_money_spent_past_year" |
                "revenue_past_year" |
                "gaming_time_solo_or_with_friends" |
                "competitive_person" |
                "birth_year" |
                "gender_identity_pronouns" |
                "country" |
                "favorite_gaming_streamers_youtubers" |
                "other_game_genres" |
                "other_hobbies" |
                "forums_post_in" => {
                    match sqlx::query!(r#"
                        INSERT INTO founder_answers (email, username, question, answer)
                        VALUES($1, $2, $3, $4)
                        ON CONFLICT (email, question)
                            DO UPDATE SET answer = EXCLUDED.answer, username = EXCLUDED.username, updated_on = now() at time zone 'utc'
                        "#,
                        email, username, key, value
                    )
                    .execute(&db)
                    .await {
                        Ok(_) => {},
                        Err(e) => { println!("{:?}", e)}
                    }
                },
                _ => {}
            }
        }
    }
}
