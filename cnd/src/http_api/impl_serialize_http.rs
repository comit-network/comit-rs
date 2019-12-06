macro_rules! _count {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + _count!($($xs)*));
}

macro_rules! impl_serialize_type_with_fields {
    ($type:ty { $($field_name:tt $(=> $field_value:ident)?),* }) => {
        impl Serialize for Http<$type> {

            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let params = _count!($($field_name)*);
                let mut state = serializer.serialize_struct("", params)?;

                $(
                  state.serialize_field($field_name, &(self.0)$(.$field_value)?)?;
                )*

                state.end()
            }
        }
    };
}

macro_rules! impl_serialize_http {
    ($type:ty) => {
        impl Serialize for Http<$type> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }
    };
}
