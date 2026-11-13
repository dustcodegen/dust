use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use minijinja::Environment;
use serde::Serialize;

/// Compiled constant templates, scoped to the current rendering worker.
type Templates = BTreeMap<String, BTreeMap<&'static str, Rc<Environment<'static>>>>;

thread_local! {
    /// Rendering contexts are fresh; only parsed templates survive a render.
    static TEMPLATES: RefCell<Templates> = const { RefCell::new(BTreeMap::new()) };
}

/// Renders a compile-time template with strict variable validation.
pub fn render_template<T>(name: &str, source: &'static str, context: T) -> String
where
    T: Serialize,
{
    let env = environment(name, source);
    let mut rendered = env
        .get_template(name)
        .expect("Dust template must be registered")
        .render(context)
        .expect("Dust template context must satisfy template variables");
    rendered.truncate(rendered.trim_end_matches('\n').len());
    rendered
}

/// Drop the cache borrow before serialization, which can render another template.
fn environment(name: &str, source: &'static str) -> Rc<Environment<'static>> {
    if let Some(env) = TEMPLATES.with(|cache| {
        cache
            .borrow()
            .get(name)
            .and_then(|sources| sources.get(source))
            .cloned()
    }) {
        return env;
    }
    let mut env = Environment::new();
    env.add_template_owned(name.to_owned(), source)
        .expect("Dust template source must be valid");
    let env = Rc::new(env);
    TEMPLATES.with(|cache| {
        cache
            .borrow_mut()
            .entry(name.to_owned())
            .or_default()
            .insert(source, Rc::clone(&env));
    });
    env
}

/// Cache reuse retains source identity, fresh contexts and recursive serialization.
#[cfg(test)]
mod tests;
