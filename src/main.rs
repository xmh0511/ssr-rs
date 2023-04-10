use salvo::prelude::*;
use salvo::serve_static::StaticDir;
use serde_json::Value;
use std::collections::HashMap;
use tera::{Context, Function, Result, Tera};
fn generate_include(tera: Tera) -> impl Function {
    move |args: &HashMap<String, Value>| -> Result<Value> {
        let Some(file_path) = args.get("path") else{
            return Err(tera::Error::msg("template path does not exist"));
        };
        match args.get("context") {
            Some(v) => {
                //println!("value === {v}");
                let context_value = v
                    .as_str()
                    .ok_or(tera::Error::msg("context must be a json object string"))?;
                let v = serde_json::from_str::<Value>(context_value)?;
                let context = Context::from_value(serde_json::json!({ "context": v }))?;
                let r = tera
                    .render(
                        file_path
                            .as_str()
                            .ok_or(tera::Error::msg("template render error"))?,
                        &context,
                    )?
                    .to_string();
                return Ok(Value::String(r));
            }
            None => {
                let context = Context::from_value(serde_json::json!({ "context": Value::Null }))?;
                let r = tera
                    .render(
                        file_path
                            .as_str()
                            .ok_or(tera::Error::msg("template render error"))?,
                        &context,
                    )?
                    .to_string();
                return Ok(Value::String(r));
            }
        }
    }
}


#[handler]
async fn render_views(req: &mut Request, res: &mut Response) {
    let Some(path) = req.param::<String>("**rest_path") else{
      res.set_status_code(StatusCode::BAD_REQUEST);
      res.render(Text::Plain("invalid request path"));
      return;
   };
    //println!("{path}");
    match Tera::new("templates/**/*") {
        Ok(mut tera) => {
            tera.register_function("include_file", generate_include(tera.clone()));
            tera.register_filter("json_decode", |v:&Value, _args:&HashMap<String, Value>|->Result<Value>{
                let v = v.as_str().ok_or(tera::Error::msg("value must be a json object string"))?;
                let v = serde_json::from_str::<Value>(v)?;
                Ok(v)
            });
            match tera.render(if path.is_empty(){"index.html"}else{&path}, &Context::default()) {
                Ok(s) => {
                    res.render(Text::Html(s));
                }
                Err(e) => {
                    res.set_status_code(StatusCode::BAD_REQUEST);
                    res.render(Text::Plain(format!("{e:?}")));
                }
            }
        }
        Err(e) => {
            res.set_status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(format!("{e:?}")));
        }
    }
}
#[tokio::main]
async fn main() {
    let config = tokio::fs::read_to_string("./config.json").await.expect("config.json not found or has invalid content");
    let config = serde_json::from_str::<Value>(&config).expect("parsing config.json occurs an error");
    let public_router = Router::with_path("public/<**>").get(
        StaticDir::new(["public"])
            .with_defaults("index.html")
            .with_listing(true),
    );
    let view_router = Router::with_path("/<**rest_path>").get(render_views);
    let router = Router::new().push(public_router);
    let router = router.push(view_router);
    Server::new(TcpListener::bind(config.get("host").expect("host not found in config.json").as_str().expect("host has none")))
        .serve(router)
        .await;
}
