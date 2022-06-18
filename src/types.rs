use crate::SqliteEventRepository;
use cqrs_es::persist::PersistedEventStore;
use cqrs_es::CqrsFramework;

/// A convenience type for a CqrsFramework backed by
/// [SqliteStore](struct.SqliteStore.html).
pub type SqliteCqrs<A> = CqrsFramework<A, PersistedEventStore<SqliteEventRepository, A>>;
