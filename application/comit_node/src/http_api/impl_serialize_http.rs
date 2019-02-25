macro_rules! _count {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + _count!($($xs)*));
}

macro_rules! impl_serialize_http {
    ($type:ty $(:= $name:tt)? { $($field_name:tt $(=> $field_value:ident)?),* }) => {
        impl Serialize for Http<$type> {

            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let params = _count!($($name)*);
                let mut state = serializer.serialize_struct("", 1 + params)?;

                #[allow(unused_variables)]
                let name = stringify!($type);
                $(let name = $name;)?
                state.serialize_field("name", name)?;

                $(
                  state.serialize_field($field_name, &(self.0)$(.$field_value)?)?;
                )*

                state.end()
            }
        }
    };
}
