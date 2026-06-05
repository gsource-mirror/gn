// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::collections::SmallSet;

use crate::attr::schema::{AllowFilesSchema, AttrKind, AttrSchema};
use crate::attr::Attr;
use crate::attr::LabelOrFile;
use crate::attr::{AllowFiles, AttrCfg};
use crate::EvalKind;
use crate::Label;
use crate::Package;

#[test]
fn test_schema_bool() {
    let a = crate::Assert::new();
    a.eq(
        "attr.bool()",
        &AttrSchema::new_test(
            AttrKind::Bool,
            Some(Attr::Bool(false)),
            false,
            AllowFilesSchema::None,
            AttrCfg::CurrentToolchain,
            String::new(),
        ),
    );

    a.fail("attr.bool(default=None)", "expected `bool`");
}

#[test]
fn test_schema_int() {
    let a = crate::Assert::new();
    a.eq(
        "attr.int(default=42, doc='An integer')",
        &AttrSchema::new_test(
            AttrKind::Int { allowed: None },
            Some(Attr::Int(42)),
            false,
            AllowFilesSchema::None,
            AttrCfg::CurrentToolchain,
            "An integer".to_string(),
        ),
    );
}

#[test]
fn test_schema_string() {
    let a = crate::Assert::new();
    let mut allowed = SmallSet::new();
    allowed.insert("foo".to_string());
    allowed.insert("bar".to_string());
    a.eq(
        "attr.string(values=['foo', 'bar'], mandatory=True)",
        &AttrSchema::new_test(
            AttrKind::String {
                allowed: Some(allowed),
            },
            None,
            false,
            AllowFilesSchema::None,
            AttrCfg::CurrentToolchain,
            String::new(),
        ),
    );
}

#[test]
fn test_schema_label() {
    let mut a = crate::Assert::new();
    a.configure(EvalKind::BzlFile(Package::from("//pkg".to_owned())));
    a.eq(
        "attr.label(allow_files=True, default=':foo')",
        &AttrSchema::new_test(
            AttrKind::Label,
            // Relative labels should be resolved relative to the bzl file, not the caller.
            Some(Attr::Label(LabelOrFile::Label(Label::new(
                Package::from("//pkg".to_owned()),
                "foo".to_owned(),
            )))),
            false,
            AllowFilesSchema::Many(AllowFiles::All),
            AttrCfg::CurrentToolchain,
            String::new(),
        ),
    );
}

#[test]
fn test_schema_string_list() {
    let a = crate::Assert::new();
    a.eq(
        "attr.string_list(allow_empty=False, mandatory=True)",
        &AttrSchema::new_test(
            AttrKind::StringList,
            None,
            true,
            AllowFilesSchema::None,
            AttrCfg::CurrentToolchain,
            String::new(),
        ),
    );
}

#[test]
fn test_schema_string_default() {
    let a = crate::Assert::new();
    a.eq(
        "attr.string(default='hello')",
        &AttrSchema::new_test(
            AttrKind::String { allowed: None },
            Some(Attr::String("hello".to_string())),
            false,
            AllowFilesSchema::None,
            AttrCfg::CurrentToolchain,
            String::new(),
        ),
    );
}

#[test]
fn test_schema_allow_files_error() {
    let a = crate::Assert::new();
    a.fail(
        "attr.label(allow_files=True, allow_single_file=True)",
        "allow_files and allow_single_file are mutually exclusive",
    );
}

#[test]
fn test_schema_mandatory_default_error() {
    let a = crate::Assert::new();
    a.fail(
        "attr.bool(mandatory=True, default=True)",
        "mandatory and default are mutually exclusive",
    );
}

#[test]
fn test_schema_default_validation_error() {
    let a = crate::Assert::new();
    a.fail(
        "attr.int(values=[1, 2], default=3)",
        "Value 3 is not in allowed set",
    );
    a.fail(
        "attr.string(values=['a', 'b'], default='c')",
        "Value \"c\" is not in allowed set",
    );
    a.fail(
        "attr.string_list(allow_empty=False)",
        "allow_empty = False requires the attribute to be mandatory or have a non-empty default value",
    );
    a.fail(
        "attr.string_list(allow_empty=False, default=[])",
        "Want non-empty list, got []",
    );
    a.fail(
        "attr.string_dict(allow_empty=False, default={})",
        "Want non-empty dict, got {}",
    );
}
