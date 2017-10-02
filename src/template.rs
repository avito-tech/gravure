use errors::*;
use liquid::{Renderable, Context, Value, Template, parse, LiquidOptions};

/// Makes paths ad urls template substitution
pub struct PathTemplate {
    template: Template,
}

impl PathTemplate {
    pub fn new(template: String) -> Result<Self, TemplateError> {
        let template = try!(parse(&template, LiquidOptions::default())
            .map_err(|e| TemplateError::Engine(e)));
        Ok(PathTemplate { template: template })
    }

    pub fn render(&self, id: u64, ext: String) -> Result<String, TemplateError> {
        let id_str = id.to_string();
        let node_id: String = id_str.clone().chars().take(2).collect();
        let ext = match ext.as_str() {
            "png" | "jpg" => ext,
            _ => return Err(TemplateError::Convert),
        };
        let mut context = Context::new();
        context.set_val("node_id", Value::Str(node_id.to_owned()));
        context.set_val("image_id", Value::Str(id_str));
        context.set_val("ext", Value::Str(ext));
        let result = try!(self.template.render(&mut context).map_err(|e| TemplateError::Engine(e)));
        Ok(result.unwrap_or(String::new()))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_path_conv() {
        let path = PathTemplate::new("http://{{node_id}}.localhost/qwer/{{image_id}}.{{ext}}"
                .to_owned())
            .unwrap();

        let res = path.render(1234, "png".to_owned()).unwrap();

        println!("{:?}", res);
    }
}
