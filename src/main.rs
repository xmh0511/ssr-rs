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
                    .ok_or(tera::Error::msg("context must be a json string"))?;
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
            match tera.render(&path, &Context::default()) {
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
    let public_router = Router::with_path("public/<**>").get(
        StaticDir::new(["public"])
            .with_defaults("index.html")
            .with_listing(true),
    );
    let view_router = Router::with_path("/<**rest_path>").get(render_views);
    let router = Router::new().push(public_router);
    let router = router.push(view_router);
    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .serve(router)
        .await;
}
