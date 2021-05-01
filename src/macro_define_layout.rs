/// This macro defines a data layout. Given such a layout, the [Field](crate::Field) or [FieldView](crate::FieldView) APIs can be used to access data based on it.
///
/// Data layouts define
/// - a name for the layout
/// - and endianness for its fields ([BigEndian](crate::BigEndian) or [LittleEndian](crate::LittleEndian))
/// - and an ordered collection of typed fields.
///
/// See [supported field types](crate#supported-field-types) for a list of supported field types.
///
/// # API
/// ```text
/// define_layout!(<<Name>>, <<Endianness>>, {
///   <<FieldName>>: <<FieldType>>,
///   <<FieldName>>: <<FieldType>>,
///   ...
/// });
/// ```
///
/// ## Field names
/// Field names can be any valid Rust identifiers, but it is recommended to avoid names that contain `storage`, `into_` or `_mut`.
/// This is because the [define_layout!] macro creates a [View class with several accessors](#struct-view) for each field that contain those identifier parts.
///
/// ## Example
/// ```
/// use binary_layout::prelude::*;
///
/// define_layout!(icmp_packet, BigEndian, {
///   packet_type: u8,
///   code: u8,
///   checksum: u16,
///   rest_of_header: [u8; 4],
///   data_section: [u8], // open ended byte array, matches until the end of the packet
/// });
/// ```
///
/// # Generated code
/// This macro will define a module for you with several members:
/// - For each field, there will be a struct containing
///   - metadata like [OFFSET](crate::FieldMetadata::OFFSET) and [SIZE](crate::SizedFieldMetadata::SIZE) as rust `const`s
///   - data accessors for the [Field](crate::Field) API
/// - The module will also contain a `View` struct that offers the [FieldView](crate::FieldView) API.
///
/// ## Metadata Example
/// ```
/// use binary_layout::prelude::*;
///
/// define_layout!(my_layout, LittleEndian, {
///   field1: u16,
///   field2: u32,
/// });
/// assert_eq!(2, my_layout::field2::OFFSET);
/// assert_eq!(4, my_layout::field2::SIZE);
/// ```
///
/// ## struct View
/// You can create views over a storage by calling `View::new`. Views can be created based on
/// - Immutable borrowed storage: `&[u8]`
/// - Mutable borrowed storage: `&mut [u8]`
/// - Owning storage: impl `AsRef<u8>` (for example: `Vec<u8>`)
///
/// The generated `View` struct will offer
/// - `View::new(storage)` to create a `View`
/// - `View::into_storage(self)` to destroy a `View` and return the storage held
///
/// and it will offer the following accessors for each field
/// - `${field_name}()`: Read access. This returns a [FieldView](crate::FieldView) instance with read access.
/// - `${field_name}_mut()`: Read access. This returns a [FieldView](crate::FieldView) instance with write access.
/// - `into_${field_name}`: Extract access. This destroys the `View` and returns a [FieldView](crate::FieldView) instance owning the storage. Mostly useful for [FieldView::extract](crate::FieldView::extract).
///
/// // TODO Show an example of using the View API
/// // TODO Show an example for generated code
/// // TODO maybe as an actual example crate?
#[macro_export]
macro_rules! define_layout {
    ($name: ident, $endianness: ident, {$($field_name: ident : $field_type: ty),* $(,)?}) => {
        #[allow(dead_code)]
        mod $name {
            #[allow(unused_imports)]
            use super::*;

            $crate::define_layout!(_impl_fields $crate::$endianness, 0, {$($field_name : $field_type),*});

            pub struct View<S> {
                storage: S,
            }
            impl <S> View<S> {
                pub fn new(storage: S) -> Self {
                    Self {storage}
                }

                pub fn into_storage(self) -> S {
                    self.storage
                }

                $crate::define_layout!(_impl_view_into {$($field_name),*});
            }
            impl <S: AsRef<[u8]>> View<S> {
                $crate::define_layout!(_impl_view_asref {$($field_name),*});
            }
            impl <S: AsMut<[u8]>> View<S> {
                $crate::define_layout!(_impl_view_asmut {$($field_name),*});
            }
        }
    };

    (_impl_fields $endianness: ty, $offset_accumulator: expr, {}) => {};
    (_impl_fields $endianness: ty, $offset_accumulator: expr, {$name: ident : $type: ty $(, $name_tail: ident : $type_tail: ty)*}) => {
        #[allow(non_camel_case_types)]
        pub type $name = $crate::Field::<$type, $endianness, $offset_accumulator>;
        $crate::define_layout!(_impl_fields $endianness, {($offset_accumulator + <$type as $crate::FieldSize>::SIZE)}, {$($name_tail : $type_tail),*});
    };

    (_impl_view_asref {}) => {};
    (_impl_view_asref {$name: ident $(, $name_tail: ident)*}) => {
        pub fn $name(&self) -> $crate::FieldView::<&[u8], $name> {
            $crate::FieldView::new(self.storage.as_ref())
        }
        $crate::define_layout!(_impl_view_asref {$($name_tail),*});
    };

    (_impl_view_asmut {}) => {};
    (_impl_view_asmut {$name: ident $(, $name_tail: ident)*}) => {
        paste::paste!{
            pub fn [<$name _mut>](&mut self) -> $crate::FieldView::<&mut [u8], $name> {
                $crate::FieldView::new(self.storage.as_mut())
            }
        }
        $crate::define_layout!(_impl_view_asmut {$($name_tail),*});
    };

    (_impl_view_into {}) => {};
    (_impl_view_into {$name: ident $(, $name_tail: ident)*}) => {
        paste::paste!{
            pub fn [<into_ $name>](self) -> $crate::FieldView::<S, $name> {
                $crate::FieldView::new(self.storage)
            }
        }
        $crate::define_layout!(_impl_view_into {$($name_tail),*});
    };
}

#[cfg(test)]
mod tests {
    use crate::{FieldMetadata, SizedFieldMetadata};

    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use std::convert::TryInto;

    fn data_region(size: usize, seed: u64) -> Vec<u8> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut res = vec![0; size];
        rng.fill_bytes(&mut res);
        res
    }

    #[test]
    fn test_layout_empty() {
        define_layout!(empty, LittleEndian, {});
    }

    mod sliceonly {
        use super::*;
        define_layout!(sliceonly, LittleEndian, { field: [u8] });

        #[test]
        fn metadata() {
            assert_eq!(0, sliceonly::field::OFFSET);
        }

        #[test]
        fn fields() {
            let mut storage = data_region(1024, 5);

            // Test initial data is read correctly
            assert_eq!(&data_region(1024, 5), sliceonly::field::data(&storage));

            // Test data can be written
            sliceonly::field::data_mut(&mut storage).copy_from_slice(&data_region(1024, 6));

            // Test reading will return changed data
            assert_eq!(&data_region(1024, 6), sliceonly::field::data(&storage));
        }

        #[test]
        fn view_readonly() {
            let storage = data_region(1024, 5);
            let view = sliceonly::View::new(&storage);

            // Test initial data is read correctly
            assert_eq!(&data_region(1024, 5), view.field().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(extracted_storage, &storage);
        }

        #[test]
        fn view_readwrite() {
            let mut storage = data_region(1024, 5);
            let mut view = sliceonly::View::new(&mut storage);

            // Test initial data is read correctly
            assert_eq!(&data_region(1024, 5), view.field().data());

            // Test data can be written
            view.field_mut()
                .data_mut()
                .copy_from_slice(&data_region(1024, 6));

            // Test reading will return changed data
            assert_eq!(&data_region(1024, 6), view.field().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage().clone();
            assert_eq!(&storage, &extracted_storage);

            // Test original storage is changed
            assert_eq!(&data_region(1024, 6), &storage);
        }

        #[test]
        fn view_vec_readonly() {
            let view = sliceonly::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(&data_region(1024, 5), view.field().data());

            // Test into_storage will return correct data
            let storage = view.into_storage();
            assert_eq!(&data_region(1024, 5), &storage);
        }

        #[test]
        fn view_vec_readwrite() {
            let mut view = sliceonly::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(&data_region(1024, 5), view.field().data());

            // Test data can be written
            view.field_mut()
                .data_mut()
                .copy_from_slice(&data_region(1024, 6));

            // Test reading will return changed data
            assert_eq!(&data_region(1024, 6), view.field().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(&data_region(1024, 6), &extracted_storage);
        }
    }

    mod noslice {
        use super::*;

        define_layout!(noslice, LittleEndian, {
            first: i8,
            second: i64,
            third: u16,
        });
        #[test]
        fn metadata() {
            assert_eq!(0, noslice::first::OFFSET);
            assert_eq!(1, noslice::first::SIZE);
            assert_eq!(1, noslice::second::OFFSET);
            assert_eq!(8, noslice::second::SIZE);
            assert_eq!(9, noslice::third::OFFSET);
            assert_eq!(2, noslice::third::SIZE);
        }

        #[test]
        fn fields() {
            let mut storage = data_region(1024, 5);

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                noslice::first::read(&storage)
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                noslice::second::read(&storage)
            );
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[9..11]).try_into().unwrap()),
                noslice::third::read(&storage)
            );

            // Test data can be written
            noslice::first::write(&mut storage, 60);
            noslice::second::write(&mut storage, -100_000_000_000);
            noslice::third::write(&mut storage, 1_000);

            // Test reading will return changed data
            assert_eq!(60, noslice::first::read(&storage));
            assert_eq!(-100_000_000_000, noslice::second::read(&storage));
            assert_eq!(1_000, noslice::third::read(&storage));
        }

        #[test]
        fn view_readonly() {
            let storage = data_region(1024, 5);
            let view = noslice::View::new(&storage);

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[9..11]).try_into().unwrap()),
                view.third().read()
            );

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(extracted_storage, &storage);
        }

        #[test]
        fn view_readwrite() {
            let mut storage = data_region(1024, 5);
            let mut view = noslice::View::new(&mut storage);

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[9..11]).try_into().unwrap()),
                view.third().read()
            );

            // Test data can be written
            view.first_mut().write(50);
            view.second_mut().write(10i64.pow(15));
            view.third_mut().write(1000);

            // Test reading will return changed data
            assert_eq!(50, view.first().read());
            assert_eq!(10i64.pow(15), view.second().read());
            assert_eq!(1000, view.third().read());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage().clone();
            assert_eq!(&storage, &extracted_storage);

            // Test original storage is actually changed
            assert_eq!(50, i8::from_le_bytes((&storage[0..1]).try_into().unwrap()));
            assert_eq!(
                10i64.pow(15),
                i64::from_le_bytes((&storage[1..9]).try_into().unwrap())
            );
            assert_eq!(
                1000,
                u16::from_le_bytes((&storage[9..11]).try_into().unwrap())
            );
        }

        #[test]
        fn view_vec_readonly() {
            let view = noslice::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[9..11]).try_into().unwrap()),
                view.third().read()
            );

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(&data_region(1024, 5), &extracted_storage);
        }

        #[test]
        fn view_vec_readwrite() {
            let mut view = noslice::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[9..11]).try_into().unwrap()),
                view.third().read()
            );

            // Test data can be written
            view.first_mut().write(50);
            view.second_mut().write(10i64.pow(15));
            view.third_mut().write(1000);

            // Test reading will return changed data
            assert_eq!(50, view.first().read());
            assert_eq!(10i64.pow(15), view.second().read());
            assert_eq!(1000, view.third().read());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(
                50,
                i8::from_le_bytes((&extracted_storage[0..1]).try_into().unwrap())
            );
            assert_eq!(
                10i64.pow(15),
                i64::from_le_bytes((&extracted_storage[1..9]).try_into().unwrap())
            );
            assert_eq!(
                1000,
                u16::from_le_bytes((&extracted_storage[9..11]).try_into().unwrap())
            );
        }
    }

    mod withslice {
        use super::*;
        define_layout!(withslice, LittleEndian, {
            first: i8,
            second: i64,
            third: [u8; 5],
            fourth: u16,
            fifth: [u8],
        });

        #[test]
        fn metadata() {
            assert_eq!(0, withslice::first::OFFSET);
            assert_eq!(1, withslice::first::SIZE);
            assert_eq!(1, withslice::second::OFFSET);
            assert_eq!(8, withslice::second::SIZE);
            assert_eq!(9, withslice::third::OFFSET);
            assert_eq!(5, withslice::third::SIZE);
            assert_eq!(14, withslice::fourth::OFFSET);
            assert_eq!(2, withslice::fourth::SIZE);
            assert_eq!(16, withslice::fifth::OFFSET);
        }

        #[test]
        fn fields() {
            let mut storage = data_region(1024, 5);

            // Test initial data is read correctly
            assert_eq!(5, withslice::third::data(&storage).len());
            assert_eq!(5, withslice::third::data_mut(&mut storage).len());
            assert_eq!(1024 - 16, withslice::fifth::data(&storage).len());
            assert_eq!(1024 - 16, withslice::fifth::data_mut(&mut storage).len());

            // Test data can be written
            withslice::first::write(&mut storage, 60);
            withslice::second::write(&mut storage, -100_000_000_000);
            withslice::third::data_mut(&mut storage).copy_from_slice(&[10, 20, 30, 40, 50]);
            withslice::fourth::write(&mut storage, 1_000);
            withslice::fifth::data_mut(&mut storage).copy_from_slice(&data_region(1024 - 16, 6));

            // Test reading will return changed data
            assert_eq!(60, withslice::first::read(&storage));
            assert_eq!(-100_000_000_000, withslice::second::read(&storage));
            assert_eq!(&[10, 20, 30, 40, 50], withslice::third::data(&storage));
            assert_eq!(1_000, withslice::fourth::read(&storage));
            assert_eq!(&data_region(1024 - 16, 6), withslice::fifth::data(&storage));
        }

        #[test]
        fn view_readonly() {
            let storage = data_region(1024, 5);
            let view = withslice::View::new(&storage);

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(&data_region(1024, 5)[9..14], view.third().data(),);
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[14..16]).try_into().unwrap()),
                view.fourth().read()
            );
            assert_eq!(&data_region(1024, 5)[16..], view.fifth().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(&storage, extracted_storage);
        }

        #[test]
        fn view_readwrite() {
            let mut storage = data_region(1024, 5);
            let mut view = withslice::View::new(&mut storage);

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(&data_region(1024, 5)[9..14], view.third().data(),);
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[14..16]).try_into().unwrap()),
                view.fourth().read()
            );
            assert_eq!(&data_region(1024, 5)[16..], view.fifth().data());

            // Test data can be written
            view.first_mut().write(50);
            view.second_mut().write(10i64.pow(15));
            view.third_mut()
                .data_mut()
                .copy_from_slice(&[10, 20, 30, 40, 50]);
            view.fourth_mut().write(1000);
            view.fifth_mut()
                .data_mut()
                .copy_from_slice(&data_region(1024, 6)[16..]);

            // Test reading will return changed data
            assert_eq!(50, view.first().read());
            assert_eq!(10i64.pow(15), view.second().read());
            assert_eq!(&[10, 20, 30, 40, 50], view.third().data());
            assert_eq!(1000, view.fourth().read());
            assert_eq!(&data_region(1024, 6)[16..], view.fifth().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage().clone();
            assert_eq!(&storage, &extracted_storage);

            // Test storage is actually changed
            assert_eq!(50, i8::from_le_bytes((&storage[0..1]).try_into().unwrap()));
            assert_eq!(
                10i64.pow(15),
                i64::from_le_bytes((&storage[1..9]).try_into().unwrap())
            );
            assert_eq!(&[10, 20, 30, 40, 50], &storage[9..14]);
            assert_eq!(
                1000,
                u16::from_le_bytes((&storage[14..16]).try_into().unwrap())
            );
            assert_eq!(&data_region(1024, 6)[16..], &storage[16..]);
        }

        #[test]
        fn view_vec_readonly() {
            let view = withslice::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(&data_region(1024, 5)[9..14], view.third().data(),);
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[14..16]).try_into().unwrap()),
                view.fourth().read()
            );
            assert_eq!(&data_region(1024, 5)[16..], view.fifth().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(&data_region(1024, 5), &extracted_storage);
        }

        #[test]
        fn view_vec_readwrite() {
            let mut view = withslice::View::new(data_region(1024, 5));

            // Test initial data is read correctly
            assert_eq!(
                i8::from_le_bytes((&data_region(1024, 5)[0..1]).try_into().unwrap()),
                view.first().read()
            );
            assert_eq!(
                i64::from_le_bytes((&data_region(1024, 5)[1..9]).try_into().unwrap()),
                view.second().read()
            );
            assert_eq!(&data_region(1024, 5)[9..14], view.third().data(),);
            assert_eq!(
                u16::from_le_bytes((&data_region(1024, 5)[14..16]).try_into().unwrap()),
                view.fourth().read()
            );
            assert_eq!(&data_region(1024, 5)[16..], view.fifth().data());

            // Test data can be written
            view.first_mut().write(50);
            view.second_mut().write(10i64.pow(15));
            view.third_mut()
                .data_mut()
                .copy_from_slice(&[10, 20, 30, 40, 50]);
            view.fourth_mut().write(1000);
            view.fifth_mut()
                .data_mut()
                .copy_from_slice(&data_region(1024, 6)[16..]);

            // Test reading will return changed data
            assert_eq!(50, view.first().read());
            assert_eq!(10i64.pow(15), view.second().read());
            assert_eq!(&[10, 20, 30, 40, 50], view.third().data());
            assert_eq!(1000, view.fourth().read());
            assert_eq!(&data_region(1024, 6)[16..], view.fifth().data());

            // Test into_storage will return correct data
            let extracted_storage = view.into_storage();
            assert_eq!(
                50,
                i8::from_le_bytes((&extracted_storage[0..1]).try_into().unwrap())
            );
            assert_eq!(
                10i64.pow(15),
                i64::from_le_bytes((&extracted_storage[1..9]).try_into().unwrap())
            );
            assert_eq!(&[10, 20, 30, 40, 50], &extracted_storage[9..14]);
            assert_eq!(
                1000,
                u16::from_le_bytes((&extracted_storage[14..16]).try_into().unwrap())
            );
            assert_eq!(&data_region(1024, 6)[16..], &extracted_storage[16..]);
        }
    }

    #[test]
    fn can_be_created_with_and_without_trailing_comma() {
        define_layout!(first, LittleEndian, { field: u8 });
        define_layout!(second, LittleEndian, {
            field: u8,
            second: u16
        });
        define_layout!(third, LittleEndian, {
            field: u8,
        });
        define_layout!(fourth, LittleEndian, {
            field: u8,
            second: u16,
        });
    }

    #[test]
    fn given_immutableview_when_extractingimmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let storage = data_region(1024, 0);
        let extracted: &[u8] = {
            let view = layout::View::new(&storage);
            view.into_tail().extract()
            // here, the view dies but the extracted reference lives on
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn given_immutableview_with_reftovec_when_extractingimmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let storage = data_region(1024, 0);
        let extracted: &[u8] = {
            let view: layout::View<&Vec<u8>> = layout::View::new(&storage);
            view.into_tail().extract()
            // here, the view dies but the extracted reference lives on
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn given_mutableview_when_extractingimmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let mut storage = data_region(1024, 0);
        let extracted: &[u8] = {
            let view: layout::View<&mut [u8]> = layout::View::new(&mut storage);
            view.into_tail().extract()
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn given_mutableview_with_reftovec_when_extractingimmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let mut storage = data_region(1024, 0);
        let extracted: &[u8] = {
            let view: layout::View<&mut Vec<u8>> = layout::View::new(&mut storage);
            view.into_tail().extract()
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn given_mutableview_when_extractingmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let mut storage = data_region(1024, 0);
        let extracted: &mut [u8] = {
            let view: layout::View<&mut [u8]> = layout::View::new(&mut storage);
            view.into_tail().extract()
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn given_mutableview_with_reftovec_when_extractingmutableref() {
        define_layout!(layout, LittleEndian, {
            field: u8,
            tail: [u8],
        });

        let mut storage = data_region(1024, 0);
        let extracted: &mut [u8] = {
            let view: layout::View<&mut Vec<u8>> = layout::View::new(&mut storage);
            view.into_tail().extract()
        };

        assert_eq!(&data_region(1024, 0)[1..], extracted);
    }

    #[test]
    fn test_little_endian() {
        define_layout!(my_layout, LittleEndian, {
            field1: u16,
            field2: i64,
        });

        let mut storage = data_region(1024, 0);
        let mut view = my_layout::View::new(&mut storage);
        view.field1_mut().write(1000);
        assert_eq!(1000, view.field1().read());
        view.field2_mut().write(10i64.pow(15));
        assert_eq!(10i64.pow(15), view.field2().read());
        assert_eq!(
            1000,
            u16::from_le_bytes((&storage[0..2]).try_into().unwrap())
        );
        assert_eq!(
            10i64.pow(15),
            i64::from_le_bytes((&storage[2..10]).try_into().unwrap())
        );
    }

    #[test]
    fn test_big_endian() {
        define_layout!(my_layout, BigEndian, {
            field1: u16,
            field2: i64,
        });

        let mut storage = data_region(1024, 0);
        let mut view = my_layout::View::new(&mut storage);
        view.field1_mut().write(1000);
        assert_eq!(1000, view.field1().read());
        view.field2_mut().write(10i64.pow(15));
        assert_eq!(10i64.pow(15), view.field2().read());
        assert_eq!(
            1000,
            u16::from_be_bytes((&storage[0..2]).try_into().unwrap())
        );
        assert_eq!(
            10i64.pow(15),
            i64::from_be_bytes((&storage[2..10]).try_into().unwrap())
        );
    }

    #[test]
    fn there_can_be_multiple_views_if_readonly() {
        define_layout!(my_layout, BigEndian, {
            field1: u16,
            field2: i64,
        });

        let storage = data_region(1024, 0);
        let view1 = my_layout::View::new(&storage);
        let view2 = my_layout::View::new(&storage);
        view1.field1().read();
        view2.field1().read();
    }
}
