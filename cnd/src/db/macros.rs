macro_rules! impl_to_sql_for_enum {
    ($enum:ident) => {
        impl<DB> ToSql<Text, DB> for $enum
        where
            DB: Backend,
            String: ToSql<Text, DB>,
        {
            fn to_sql<W: std::io::Write>(&self, out: &mut Output<'_, W, DB>) -> serialize::Result {
                let s = self.to_string();
                s.to_sql(out)
            }
        }
    };
}

macro_rules! impl_from_sql_for_enum {
    ($enum:ident) => {
        impl<DB> FromSql<Text, DB> for $enum
        where
            DB: Backend,
            String: FromSql<Text, DB>,
        {
            fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
                let s = String::from_sql(bytes)?;
                let variant = Self::from_str(s.as_ref())?;

                Ok(variant)
            }
        }
    };
}
