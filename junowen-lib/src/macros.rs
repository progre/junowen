#[macro_export]
macro_rules! hook {
    ($addr:expr, $hook:ident, $type:ty) => {
        pub fn $hook(&self, target: $type) -> ($type, ApplyFn) {
            let addr = $addr;
            unsafe {
                transmute::<(usize, ApplyFn), ($type, ApplyFn)>(self.hook_call(addr, target as _))
            }
        }
    };
}

#[macro_export]
macro_rules! hook_todo {
    ($addr:expr, $hook:ident, $type:ty) => {
        pub fn $hook(&self, _target: $type) -> ($type, ApplyFn) {
            todo!()
        }
    };
}

#[macro_export]
macro_rules! u16_prop {
    ($addr:expr, $getter:ident) => {
        pub fn $getter(&self) -> Result<u16> {
            self.memory_accessor.read_u16($addr)
        }
    };

    ($addr:expr, $getter:ident, $setter:ident) => {
        $crate::u16_prop!($addr, $getter);
        pub fn $setter(&mut self, value: u16) -> Result<()> {
            self.memory_accessor.write_u16($addr, value)
        }
    };
}

#[macro_export]
macro_rules! u32_prop {
    ($addr:expr, $getter:ident) => {
        pub fn $getter(&self) -> Result<u32> {
            self.memory_accessor.read_u32($addr)
        }
    };

    ($addr:expr, $getter:ident, $setter:ident) => {
        $crate::u32_prop!($addr, $getter);
        pub fn $setter(&mut self, value: u32) -> Result<()> {
            self.memory_accessor.write_u32($addr, value)
        }
    };
}

#[macro_export]
macro_rules! u32_prop_todo {
    ($addr:expr, $getter:ident) => {
        pub fn $getter(&self) -> Result<u32> {
            todo!("u32_prop");
        }
    };

    ($addr:expr, $getter:ident, $setter:ident) => {
        $crate::u32_prop_todo!($addr, $getter);
        pub fn $setter(&mut self, value: u32) -> Result<()> {
            todo!("u32_prop");
        }
    };
}

#[macro_export]
macro_rules! pointer {
    ($addr:expr, $getter:ident, $type:ty) => {
        pub fn $getter(&self) -> &'static $type {
            self.pointer($addr).unwrap()
        }
    };
    ($addr:expr, $getter:ident, $getter_mut:ident, $type:ty) => {
        pointer!($addr, $getter, $type);
        pub fn $getter_mut(&mut self) -> &'static mut $type {
            self.pointer_mut($addr).unwrap()
        }
    };
}

#[macro_export]
macro_rules! ptr_opt {
    ($addr:expr, $getter:ident, $type:ty) => {
        pub fn $getter(&self) -> Option<&'static $type> {
            self.pointer($addr)
        }
    };
    ($addr:expr, $getter:ident, $getter_mut:ident, $type:ty) => {
        ptr_opt!($addr, $getter, $type);
        pub fn $getter_mut(&mut self) -> Option<&'static mut $type> {
            self.pointer_mut($addr)
        }
    };
}

#[macro_export]
macro_rules! value_ref {
    ($addr:expr, $getter:ident, $type:ty) => {
        pub fn $getter(&self) -> &'static $type {
            self.value_ref($addr)
        }
    };
    ($addr:expr, $getter:ident, $getter_mut:ident, $type:ty) => {
        value_ref!($addr, $getter, $type);
        pub fn $getter_mut(&mut self) -> &'static mut $type {
            self.value_mut($addr)
        }
    };
}
