use std::convert::TryFrom;
use std::ffi::CString;
use std::iter::FromIterator;

use crate::{
    sys::{
        self, WSPutArgCount, WSPutInteger64, WSPutReal64, WSPutUTF8String,
        WSPutUTF8Symbol,
    },
    Error, Link,
};

impl Link {
    /// TODO: Augment this function with a `put_type()` method which takes a
    ///       (non-exhaustive) enum value.
    ///
    /// *WSTP C API Documentation:* [`WSPutType()`](https://reference.wolfram.com/language/ref/c/WSPutType.html)
    pub fn put_raw_type(&mut self, type_: i32) -> Result<(), Error> {
        if unsafe { sys::WSPutType(self.raw_link, type_) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Atoms
    //==================================

    /// *WSTP C API Documentation:* [`WSPutUTF8String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8String.html)
    pub fn put_str(&mut self, string: &str) -> Result<(), Error> {
        // TODO: Optimization:
        //     This intermediate CString allocation may not actually be necessary. Because
        //     WSPutUTF8String() takes a pointer + length pair, it's possible it doesn't
        //     require that the string be NULL terminated. I'm not confident that is the
        //     case though, and it isn't explicitly documented one way or the other.
        //     Investigate this in the WSTP sources, and fix this if possible. If fixed,
        //     be sure to include this assertion (`str`'s can contain NULL bytes, and
        //     I have much less confidence that older parts of WSTP are strict about not
        //     using strlen() on strings internally).
        //
        //         assert!(!string.bytes().any(|byte| byte == 0));
        let c_string = CString::new(string).unwrap();

        let len = i32::try_from(c_string.as_bytes().len()).expect("usize overflows i32");
        let ptr = c_string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8String(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8Symbol.html)
    pub fn put_symbol(&mut self, symbol: &str) -> Result<(), Error> {
        // FIXME:
        //     Is this extra allocation necessary?WSPutUTF8Symbol doesn't seem to require
        //     that the data contains a NULL terminator, so we should be able to just
        //     pass a pointer to `symbol`'s data.
        let c_string = CString::new(symbol).unwrap();

        let len = i32::try_from(c_string.as_bytes().len()).expect("usize overflows i32");
        let ptr = c_string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8Symbol(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Functions
    //==================================

    /// Begin putting a function onto this link.
    ///
    /// # Examples
    ///
    /// Put the expression `{1, 2, 3}` on the link:
    ///
    /// ```
    /// # use wstp::Link;
    /// # fn test() -> Result<(), wstp::Error> {
    /// let mut link = Link::new_loopback()?;
    ///
    /// link.put_function("System`List", 3)?;
    /// link.put_i64(1)?;
    /// link.put_i64(2)?;
    /// link.put_i64(3)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Put the expression `foo["a"]["b"]` on the link:
    ///
    /// ```
    /// # use wstp::Link;
    /// # fn test() -> Result<wolfram_expr::Expr, wstp::Error> {
    /// let mut link = Link::new_loopback()?;
    ///
    /// link.put_function(None, 1)?;
    /// link.put_function("Global`foo", 1)?;
    /// link.put_str("a")?;
    /// link.put_str("b")?;
    /// # link.get_expr()
    /// # }
    ///
    /// # use wolfram_expr::{Expr, Symbol};
    /// # assert_eq!(test().unwrap(), Expr::normal(
    /// #     Expr::normal(Symbol::new("Global`foo"), vec![Expr::string("a")]),
    /// #     vec![Expr::string("b")]
    /// # ))
    /// ```
    pub fn put_function<'h, H: Into<Option<&'h str>>>(
        &mut self,
        head: H,
        count: usize,
    ) -> Result<(), Error> {
        self.put_raw_type(i32::from(sys::WSTKFUNC))?;
        self.put_arg_count(count)?;

        if let Some(head) = head.into() {
            self.put_symbol(head)?;
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutArgCount()`](https://reference.wolfram.com/language/ref/c/WSPutArgCount.html)
    pub fn put_arg_count(&mut self, count: usize) -> Result<(), Error> {
        let count: i32 = i32::try_from(count).map_err(|err| {
            Error::custom(format!(
                "put_arg_count: Error converting usize to i32: {}",
                err
            ))
        })?;

        if unsafe { WSPutArgCount(self.raw_link, count) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Numerics
    //==================================

    /// *WSTP C API Documentation:* [`WSPutInteger64()`](https://reference.wolfram.com/language/ref/c/WSPutInteger64.html)
    pub fn put_i64(&mut self, value: i64) -> Result<(), Error> {
        if unsafe { WSPutInteger64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutReal64()`](https://reference.wolfram.com/language/ref/c/WSPutReal64.html)
    pub fn put_f64(&mut self, value: f64) -> Result<(), Error> {
        if unsafe { WSPutReal64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// Put a multidimensional array of [`i64`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutInteger64Array()`](https://reference.wolfram.com/language/ref/c/WSPutInteger64Array.html)
    pub fn put_i64_array(
        &mut self,
        data: &[i64],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = Vec::from_iter(
            dimensions
                .iter()
                .map(|&val| i32::try_from(val).expect("i32 overflows usize")),
        );

        let result = unsafe {
            sys::WSPutInteger64Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Put a multidimensional array of [`f64`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutReal64Array()`](https://reference.wolfram.com/language/ref/c/WSPutReal64Array.html)
    pub fn put_f64_array(
        &mut self,
        data: &[f64],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = Vec::from_iter(
            dimensions
                .iter()
                .map(|&val| i32::try_from(val).expect("i32 overflows usize")),
        );

        let result = unsafe {
            sys::WSPutReal64Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }
}
