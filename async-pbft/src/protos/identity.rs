// This file is generated by rust-protobuf 2.27.1. Do not edit
// @generated

// https://github.com/rust-lang/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(unused_attributes)]
#![cfg_attr(rustfmt, rustfmt::skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_imports)]
#![allow(unused_results)]
//! Generated file from `identity.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
// const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_27_1;

#[derive(PartialEq,Clone,Default)]
pub struct Policy {
    // message fields
    pub name: ::std::string::String,
    pub entries: ::protobuf::RepeatedField<Policy_Entry>,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a Policy {
    fn default() -> &'a Policy {
        <Policy as ::protobuf::Message>::default_instance()
    }
}

impl Policy {
    pub fn new() -> Policy {
        ::std::default::Default::default()
    }

    // string name = 1;


    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn clear_name(&mut self) {
        self.name.clear();
    }

    // Param is passed by value, moved
    pub fn set_name(&mut self, v: ::std::string::String) {
        self.name = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_name(&mut self) -> &mut ::std::string::String {
        &mut self.name
    }

    // Take field
    pub fn take_name(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.name, ::std::string::String::new())
    }

    // repeated .Policy.Entry entries = 2;


    pub fn get_entries(&self) -> &[Policy_Entry] {
        &self.entries
    }
    pub fn clear_entries(&mut self) {
        self.entries.clear();
    }

    // Param is passed by value, moved
    pub fn set_entries(&mut self, v: ::protobuf::RepeatedField<Policy_Entry>) {
        self.entries = v;
    }

    // Mutable pointer to the field.
    pub fn mut_entries(&mut self) -> &mut ::protobuf::RepeatedField<Policy_Entry> {
        &mut self.entries
    }

    // Take field
    pub fn take_entries(&mut self) -> ::protobuf::RepeatedField<Policy_Entry> {
        ::std::mem::replace(&mut self.entries, ::protobuf::RepeatedField::new())
    }
}

impl ::protobuf::Message for Policy {
    fn is_initialized(&self) -> bool {
        for v in &self.entries {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.name)?;
                },
                2 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.entries)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.name.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.name);
        }
        for value in &self.entries {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.name.is_empty() {
            os.write_string(1, &self.name)?;
        }
        for v in &self.entries {
            os.write_tag(2, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> Policy {
        Policy::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "name",
                |m: &Policy| { &m.name },
                |m: &mut Policy| { &mut m.name },
            ));
            fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Policy_Entry>>(
                "entries",
                |m: &Policy| { &m.entries },
                |m: &mut Policy| { &mut m.entries },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<Policy>(
                "Policy",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static Policy {
        static instance: ::protobuf::rt::LazyV2<Policy> = ::protobuf::rt::LazyV2::INIT;
        instance.get(Policy::new)
    }
}

impl ::protobuf::Clear for Policy {
    fn clear(&mut self) {
        self.name.clear();
        self.entries.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Policy {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Policy_Entry {
    // message fields
    pub field_type: Policy_EntryType,
    pub key: ::std::string::String,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a Policy_Entry {
    fn default() -> &'a Policy_Entry {
        <Policy_Entry as ::protobuf::Message>::default_instance()
    }
}

impl Policy_Entry {
    pub fn new() -> Policy_Entry {
        ::std::default::Default::default()
    }

    // .Policy.EntryType type = 1;


    pub fn get_field_type(&self) -> Policy_EntryType {
        self.field_type
    }
    pub fn clear_field_type(&mut self) {
        self.field_type = Policy_EntryType::ENTRY_TYPE_UNSET;
    }

    // Param is passed by value, moved
    pub fn set_field_type(&mut self, v: Policy_EntryType) {
        self.field_type = v;
    }

    // string key = 2;


    pub fn get_key(&self) -> &str {
        &self.key
    }
    pub fn clear_key(&mut self) {
        self.key.clear();
    }

    // Param is passed by value, moved
    pub fn set_key(&mut self, v: ::std::string::String) {
        self.key = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_key(&mut self) -> &mut ::std::string::String {
        &mut self.key
    }

    // Take field
    pub fn take_key(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.key, ::std::string::String::new())
    }
}

impl ::protobuf::Message for Policy_Entry {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_proto3_enum_with_unknown_fields_into(wire_type, is, &mut self.field_type, 1, &mut self.unknown_fields)?
                },
                2 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.key)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if self.field_type != Policy_EntryType::ENTRY_TYPE_UNSET {
            my_size += ::protobuf::rt::enum_size(1, self.field_type);
        }
        if !self.key.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.key);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if self.field_type != Policy_EntryType::ENTRY_TYPE_UNSET {
            os.write_enum(1, ::protobuf::ProtobufEnum::value(&self.field_type))?;
        }
        if !self.key.is_empty() {
            os.write_string(2, &self.key)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> Policy_Entry {
        Policy_Entry::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeEnum<Policy_EntryType>>(
                "type",
                |m: &Policy_Entry| { &m.field_type },
                |m: &mut Policy_Entry| { &mut m.field_type },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "key",
                |m: &Policy_Entry| { &m.key },
                |m: &mut Policy_Entry| { &mut m.key },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<Policy_Entry>(
                "Policy.Entry",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static Policy_Entry {
        static instance: ::protobuf::rt::LazyV2<Policy_Entry> = ::protobuf::rt::LazyV2::INIT;
        instance.get(Policy_Entry::new)
    }
}

impl ::protobuf::Clear for Policy_Entry {
    fn clear(&mut self) {
        self.field_type = Policy_EntryType::ENTRY_TYPE_UNSET;
        self.key.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Policy_Entry {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Policy_Entry {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub enum Policy_EntryType {
    ENTRY_TYPE_UNSET = 0,
    PERMIT_KEY = 1,
    DENY_KEY = 2,
}

impl ::protobuf::ProtobufEnum for Policy_EntryType {
    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<Policy_EntryType> {
        match value {
            0 => ::std::option::Option::Some(Policy_EntryType::ENTRY_TYPE_UNSET),
            1 => ::std::option::Option::Some(Policy_EntryType::PERMIT_KEY),
            2 => ::std::option::Option::Some(Policy_EntryType::DENY_KEY),
            _ => ::std::option::Option::None
        }
    }

    fn values() -> &'static [Self] {
        static values: &'static [Policy_EntryType] = &[
            Policy_EntryType::ENTRY_TYPE_UNSET,
            Policy_EntryType::PERMIT_KEY,
            Policy_EntryType::DENY_KEY,
        ];
        values
    }

    fn enum_descriptor_static() -> &'static ::protobuf::reflect::EnumDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::EnumDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            ::protobuf::reflect::EnumDescriptor::new_pb_name::<Policy_EntryType>("Policy.EntryType", file_descriptor_proto())
        })
    }
}

impl ::std::marker::Copy for Policy_EntryType {
}

impl ::std::default::Default for Policy_EntryType {
    fn default() -> Self {
        Policy_EntryType::ENTRY_TYPE_UNSET
    }
}

impl ::protobuf::reflect::ProtobufValue for Policy_EntryType {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Enum(::protobuf::ProtobufEnum::descriptor(self))
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct PolicyList {
    // message fields
    pub policies: ::protobuf::RepeatedField<Policy>,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a PolicyList {
    fn default() -> &'a PolicyList {
        <PolicyList as ::protobuf::Message>::default_instance()
    }
}

impl PolicyList {
    pub fn new() -> PolicyList {
        ::std::default::Default::default()
    }

    // repeated .Policy policies = 1;


    pub fn get_policies(&self) -> &[Policy] {
        &self.policies
    }
    pub fn clear_policies(&mut self) {
        self.policies.clear();
    }

    // Param is passed by value, moved
    pub fn set_policies(&mut self, v: ::protobuf::RepeatedField<Policy>) {
        self.policies = v;
    }

    // Mutable pointer to the field.
    pub fn mut_policies(&mut self) -> &mut ::protobuf::RepeatedField<Policy> {
        &mut self.policies
    }

    // Take field
    pub fn take_policies(&mut self) -> ::protobuf::RepeatedField<Policy> {
        ::std::mem::replace(&mut self.policies, ::protobuf::RepeatedField::new())
    }
}

impl ::protobuf::Message for PolicyList {
    fn is_initialized(&self) -> bool {
        for v in &self.policies {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.policies)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        for value in &self.policies {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        for v in &self.policies {
            os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> PolicyList {
        PolicyList::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Policy>>(
                "policies",
                |m: &PolicyList| { &m.policies },
                |m: &mut PolicyList| { &mut m.policies },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<PolicyList>(
                "PolicyList",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static PolicyList {
        static instance: ::protobuf::rt::LazyV2<PolicyList> = ::protobuf::rt::LazyV2::INIT;
        instance.get(PolicyList::new)
    }
}

impl ::protobuf::Clear for PolicyList {
    fn clear(&mut self) {
        self.policies.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for PolicyList {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for PolicyList {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Role {
    // message fields
    pub name: ::std::string::String,
    pub policy_name: ::std::string::String,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a Role {
    fn default() -> &'a Role {
        <Role as ::protobuf::Message>::default_instance()
    }
}

impl Role {
    pub fn new() -> Role {
        ::std::default::Default::default()
    }

    // string name = 1;


    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn clear_name(&mut self) {
        self.name.clear();
    }

    // Param is passed by value, moved
    pub fn set_name(&mut self, v: ::std::string::String) {
        self.name = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_name(&mut self) -> &mut ::std::string::String {
        &mut self.name
    }

    // Take field
    pub fn take_name(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.name, ::std::string::String::new())
    }

    // string policy_name = 2;


    pub fn get_policy_name(&self) -> &str {
        &self.policy_name
    }
    pub fn clear_policy_name(&mut self) {
        self.policy_name.clear();
    }

    // Param is passed by value, moved
    pub fn set_policy_name(&mut self, v: ::std::string::String) {
        self.policy_name = v;
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_policy_name(&mut self) -> &mut ::std::string::String {
        &mut self.policy_name
    }

    // Take field
    pub fn take_policy_name(&mut self) -> ::std::string::String {
        ::std::mem::replace(&mut self.policy_name, ::std::string::String::new())
    }
}

impl ::protobuf::Message for Role {
    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.name)?;
                },
                2 => {
                    ::protobuf::rt::read_singular_proto3_string_into(wire_type, is, &mut self.policy_name)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if !self.name.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.name);
        }
        if !self.policy_name.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.policy_name);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        if !self.name.is_empty() {
            os.write_string(1, &self.name)?;
        }
        if !self.policy_name.is_empty() {
            os.write_string(2, &self.policy_name)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> Role {
        Role::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "name",
                |m: &Role| { &m.name },
                |m: &mut Role| { &mut m.name },
            ));
            fields.push(::protobuf::reflect::accessor::make_simple_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                "policy_name",
                |m: &Role| { &m.policy_name },
                |m: &mut Role| { &mut m.policy_name },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<Role>(
                "Role",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static Role {
        static instance: ::protobuf::rt::LazyV2<Role> = ::protobuf::rt::LazyV2::INIT;
        instance.get(Role::new)
    }
}

impl ::protobuf::Clear for Role {
    fn clear(&mut self) {
        self.name.clear();
        self.policy_name.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Role {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Role {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct RoleList {
    // message fields
    pub roles: ::protobuf::RepeatedField<Role>,
    // special fields
    pub unknown_fields: ::protobuf::UnknownFields,
    pub cached_size: ::protobuf::CachedSize,
}

impl<'a> ::std::default::Default for &'a RoleList {
    fn default() -> &'a RoleList {
        <RoleList as ::protobuf::Message>::default_instance()
    }
}

impl RoleList {
    pub fn new() -> RoleList {
        ::std::default::Default::default()
    }

    // repeated .Role roles = 1;


    pub fn get_roles(&self) -> &[Role] {
        &self.roles
    }
    pub fn clear_roles(&mut self) {
        self.roles.clear();
    }

    // Param is passed by value, moved
    pub fn set_roles(&mut self, v: ::protobuf::RepeatedField<Role>) {
        self.roles = v;
    }

    // Mutable pointer to the field.
    pub fn mut_roles(&mut self) -> &mut ::protobuf::RepeatedField<Role> {
        &mut self.roles
    }

    // Take field
    pub fn take_roles(&mut self) -> ::protobuf::RepeatedField<Role> {
        ::std::mem::replace(&mut self.roles, ::protobuf::RepeatedField::new())
    }
}

impl ::protobuf::Message for RoleList {
    fn is_initialized(&self) -> bool {
        for v in &self.roles {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.roles)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        for value in &self.roles {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::ProtobufResult<()> {
        for v in &self.roles {
            os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &dyn (::std::any::Any) {
        self as &dyn (::std::any::Any)
    }
    fn as_any_mut(&mut self) -> &mut dyn (::std::any::Any) {
        self as &mut dyn (::std::any::Any)
    }
    fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn (::std::any::Any)> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        Self::descriptor_static()
    }

    fn new() -> RoleList {
        RoleList::new()
    }

    fn descriptor_static() -> &'static ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::LazyV2<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::LazyV2::INIT;
        descriptor.get(|| {
            let mut fields = ::std::vec::Vec::new();
            fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Role>>(
                "roles",
                |m: &RoleList| { &m.roles },
                |m: &mut RoleList| { &mut m.roles },
            ));
            ::protobuf::reflect::MessageDescriptor::new_pb_name::<RoleList>(
                "RoleList",
                fields,
                file_descriptor_proto()
            )
        })
    }

    fn default_instance() -> &'static RoleList {
        static instance: ::protobuf::rt::LazyV2<RoleList> = ::protobuf::rt::LazyV2::INIT;
        instance.get(RoleList::new)
    }
}

impl ::protobuf::Clear for RoleList {
    fn clear(&mut self) {
        self.roles.clear();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for RoleList {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for RoleList {
    fn as_ref(&self) -> ::protobuf::reflect::ReflectValueRef {
        ::protobuf::reflect::ReflectValueRef::Message(self)
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x0eidentity.proto\"\xd6\x01\n\x06Policy\x12\x14\n\x04name\x18\x01\x20\
    \x01(\tR\x04nameB\0\x12)\n\x07entries\x18\x02\x20\x03(\x0b2\r.Policy.Ent\
    ryR\x07entriesB\0\x1aF\n\x05Entry\x12'\n\x04type\x18\x01\x20\x01(\x0e2\
    \x11.Policy.EntryTypeR\x04typeB\0\x12\x12\n\x03key\x18\x02\x20\x01(\tR\
    \x03keyB\0:\0\"A\n\tEntryType\x12\x14\n\x10ENTRY_TYPE_UNSET\x10\0\x12\
    \x0e\n\nPERMIT_KEY\x10\x01\x12\x0c\n\x08DENY_KEY\x10\x02\x1a\0:\0\"5\n\n\
    PolicyList\x12%\n\x08policies\x18\x01\x20\x03(\x0b2\x07.PolicyR\x08polic\
    iesB\0:\0\"A\n\x04Role\x12\x14\n\x04name\x18\x01\x20\x01(\tR\x04nameB\0\
    \x12!\n\x0bpolicy_name\x18\x02\x20\x01(\tR\npolicyNameB\0:\0\"+\n\x08Rol\
    eList\x12\x1d\n\x05roles\x18\x01\x20\x03(\x0b2\x05.RoleR\x05rolesB\0:\0B\
    \0b\x06proto3\
";

static file_descriptor_proto_lazy: ::protobuf::rt::LazyV2<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::LazyV2::INIT;

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::Message::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    file_descriptor_proto_lazy.get(|| {
        parse_descriptor_proto()
    })
}
