use juniper::graphql_object;

use super::Context;

#[allow(clippy::module_name_repetitions)]
pub struct RootMutation;

#[graphql_object(context = Context)]
impl RootMutation {
    fn foo() -> &str {
        "hello"
    }
}

/*
#[graphql_object(context = Context, scalar = S)]
impl<S: ScalarValue + Display> Mutation {
  fn createHuman(context: &Context, new_human: NewHuman) -> FieldResult<Human, S> {
    let db = context
      .pool
      .get_connection()
      .map_err(|e| e.map_scalar_value())?;
    let human: Human = db
      .insert_human(&new_human)
      .map_err(|e| e.map_scalar_value())?;
    Ok(human)
  }
}
*/
