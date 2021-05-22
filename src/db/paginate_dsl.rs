use diesel::query_dsl::methods::{LimitDsl, OffsetDsl};
use juniper::GraphQLInputObject;

// TODO: any way to derive or define this in the gql submodule?
#[derive(GraphQLInputObject)]
pub struct Pagination {
    pub take: Option<i32>,
    pub skip: Option<i32>,
}

#[allow(clippy::module_name_repetitions)]
pub trait PaginateDsl {
    type Output;
    fn paginate(self, pagination: Option<Pagination>) -> Self::Output;
}

impl<T> PaginateDsl for T
where
    T: OffsetDsl,
    T::Output: LimitDsl,
{
    type Output = <T::Output as LimitDsl>::Output;
    fn paginate(self, pagination: Option<Pagination>) -> Self::Output {
        let mut skip = 0;
        let mut take = 10;
        if let Some(pagination) = pagination {
            skip = pagination.skip.unwrap_or(skip);
            take = pagination.take.unwrap_or(take);
        }
        self.offset(skip.into()).limit(take.into())
    }
}
