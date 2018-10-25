#[derive(Debug, PartialEq)]
pub enum QueryMatchResult {
    Yes { confirmations_needed: u32 },
    No,
}

impl QueryMatchResult {
    pub fn yes() -> Self {
        QueryMatchResult::Yes {
            confirmations_needed: 0,
        }
    }
    pub fn yes_with_confirmations(confirmations_needed: u32) -> Self {
        QueryMatchResult::Yes {
            confirmations_needed,
        }
    }
    pub fn no() -> Self {
        QueryMatchResult::No
    }

    pub fn or(self, other: QueryMatchResult) -> QueryMatchResult {
        match self {
            QueryMatchResult::Yes {
                confirmations_needed,
            } => match other {
                QueryMatchResult::Yes {
                    confirmations_needed: other_confirmations_needed,
                } => QueryMatchResult::Yes {
                    confirmations_needed: confirmations_needed.max(other_confirmations_needed),
                },
                QueryMatchResult::No => QueryMatchResult::Yes {
                    confirmations_needed,
                },
            },
            QueryMatchResult::No => match other {
                QueryMatchResult::Yes {
                    confirmations_needed,
                } => QueryMatchResult::Yes {
                    confirmations_needed,
                },
                QueryMatchResult::No => QueryMatchResult::No,
            },
        }
    }

    pub fn and(self, other: QueryMatchResult) -> QueryMatchResult {
        match self {
            QueryMatchResult::Yes {
                confirmations_needed,
            } => match other {
                QueryMatchResult::Yes {
                    confirmations_needed: other_confirmations_needed,
                } => QueryMatchResult::Yes {
                    confirmations_needed: confirmations_needed.max(other_confirmations_needed),
                },
                _ => QueryMatchResult::no(),
            },
            _ => QueryMatchResult::no(),
        }
    }
}

pub trait Matches<T> {
    fn matches<P>(&self, predicate: P) -> QueryMatchResult
    where
        P: Fn(&T) -> bool;
}

impl<T> Matches<T> for Option<T> {
    fn matches<P>(&self, predicate: P) -> QueryMatchResult
    where
        P: Fn(&T) -> bool,
    {
        match self {
            Some(this) if predicate(this) => QueryMatchResult::yes(),
            _ => QueryMatchResult::no(),
        }
    }
}

pub trait IsEqualTo<T, U> {
    fn is_equal_to<O>(&self, other: O) -> QueryMatchResult
    where
        O: Fn() -> U;
}

impl<'a, T: PartialEq + 'a> IsEqualTo<T, &'a T> for Option<T> {
    fn is_equal_to<O>(&self, other: O) -> QueryMatchResult
    where
        O: Fn() -> &'a T,
    {
        match self {
            Some(this) if this == other() => QueryMatchResult::yes(),
            _ => QueryMatchResult::no(),
        }
    }
}

impl<T: PartialEq> IsEqualTo<T, T> for Option<T> {
    fn is_equal_to<O>(&self, other: O) -> QueryMatchResult
    where
        O: Fn() -> T,
    {
        match self {
            Some(this) if this == &other() => QueryMatchResult::yes(),
            _ => QueryMatchResult::no(),
        }
    }
}

impl<'a, T: PartialEq + 'a> IsEqualTo<T, &'a Option<T>> for Option<T> {
    fn is_equal_to<O>(&self, other: O) -> QueryMatchResult
    where
        O: Fn() -> &'a Option<T>,
    {
        match (self, other().as_ref()) {
            (Some(this), Some(other)) if this == other => QueryMatchResult::yes(),
            _ => QueryMatchResult::no(),
        }
    }
}

impl<T: PartialEq> IsEqualTo<T, Option<T>> for Option<T> {
    fn is_equal_to<O>(&self, other: O) -> QueryMatchResult
    where
        O: Fn() -> Option<T>,
    {
        match (self, other()) {
            (Some(this), Some(ref other)) if this == other => QueryMatchResult::yes(),
            _ => QueryMatchResult::no(),
        }
    }
}
