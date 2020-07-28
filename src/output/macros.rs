#[macro_export]
macro_rules! render_field_type_converter {
    (bool, $value:ident) => {
        $value == 1
    };
    (u32, $value:ident) => {
        $value
    };
    ( enum:$field_type:ident, $value:ident) => {
        super::super::$field_type::from($value)
    };
}

#[macro_export]
macro_rules! render_field_type {
    ($field_type:ident) => {
        $field_type
    };
    ( enum:$field_type:ident) => {
        super::super::$field_type
    };
}

#[macro_export]
macro_rules! create_getters {
    ($($r_field:ident($r_field_mask:expr, $($r_field_type:tt)*), )*) => {
        $(
        #[allow(non_snake_case)]
        pub fn $r_field(&self) -> render_field_type!($($r_field_type)*) {
            let mask : u32 = $r_field_mask;
            let raw_value = (self.value & mask >> mask.trailing_zeros());
            render_field_type_converter!($($r_field_type)*, raw_value)
        }
        )*
    };
}

#[macro_export]
macro_rules! create_setters {
    ($($w_field:ident($w_field_mask:expr, $($w_field_type:tt)*), )*) => {
        $(
        #[allow(non_snake_case)]
        pub fn $w_field(&mut self, value: render_field_type!($($w_field_type)*)) {
            let mask : u32 = $w_field_mask;
            self.value = (self.value & !mask) | (((value as u32)  << mask.trailing_zeros() ) & mask);
            self.mask = self.mask | mask;
        }
        )*
    };
}

#[macro_export]
macro_rules! create_reg {
    ($peripheral:ident::$reg:ident($reg_size:ident) =>
            $(RW{$($rw_tts:tt)+})?
            $(R{$($r_tts:tt)+})?
            $(W{$($w_tts:tt)+})?
    ) => {
        pub mod $reg{
            pub fn new() -> Writer {
                Writer::new()
            }
            pub fn read() -> Reader {
                Reader::new()
            }

            pub struct Writer{
                value: u32,
                mask: u32,
            }
            impl Writer{
                pub fn new() -> Writer {
                    Writer{value: 0, mask: 0}
                }
                pub fn write(&self) {
                    unsafe{
                        let mut value = core::ptr::read_volatile(&crate::peripherals::$peripheral.$reg);
                        value = (value & !self.mask) | (self.value & self.mask);
                        core::ptr::write_volatile(&mut crate::peripherals::$peripheral.$reg, value);
                    }
                }
                /*
                Implementation of setters
                 */
                 create_setters!{$($($rw_tts)*)? $($($w_tts)*)?}
            }
            pub struct Reader{
                value: u32
            }
            impl Reader{
                pub fn new() -> Reader {
                    unsafe{
                        Reader{ value: core::ptr::read_volatile(&crate::peripherals::$peripheral.$reg)}
                    }
                }
                /*
                Implementation of accessors
                 */
                create_getters!($($($r_tts)*)? $($($rw_tts)*)?);
            }
        }
        //create_ordered_reg!{$peripheral::$reg($reg_size) => R{$($($r_tts)*)? $($($rw_tts)*)?} W{$($($rw_tts)*)? $($($w_tts)*)?}}
    };
}
