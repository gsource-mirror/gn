// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::Module;
use starlark::values::list::UnpackList;
use starlark::values::none::NoneOr;
use starlark::values::{FrozenHeap, UnpackValue, Value, ValueLike};
use starlark::collections::SmallSet;

use attr::cfg::AttrCfg;
use attr::schema::{AllowFilesSchema, AttrSchema, AttrKind};
use attr::value::AttrValue;
use attr::{AllowFiles, Attr, LabelOrFile};
use types::{File, Label, PackageRef, PathResolver, LabelRef, TargetRef};

// Mocks from testutils
use testutils::{FakeSession, FakeTarget, FakeTargetRef, FakeEvalContext, Assert};

mod attr_globals {
    use super::FakeEvalContext;
    use attr::EvalContext;
    attr::declare_attr_module!(FakeEvalContext);
}
use attr_globals::AttrModule;

fn new_assert_with_attr(package: &str) -> (Box<FakeEvalContext>, Assert) {
    let context = Box::new(FakeEvalContext::new(package));
    let mut assert = Assert::new();
    assert.globals_add(|builder| {
        builder.set("attr", AttrModule);
    });
    let context_ptr = &*context as *const FakeEvalContext;
    assert.setup_eval(move |eval| {
        // Safety: context is boxed and returned to the caller, so it remains at the same heap
        // address and outlives any evaluation running inside the caller test function.
        let context_ref: &'static FakeEvalContext = unsafe { &*context_ptr };
        let extra: &dyn starlark::any::AnyLifetime = context_ref;
        eval.extra = Some(extra);
    });
    (context, assert)
}

#[test]
fn test_schema_bool() {
    let (_ctx, a) = new_assert_with_attr("//");

    a.eq(
        "attr.bool()",
        &AttrSchema::new_for_testing(
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
    let (_ctx, a) = new_assert_with_attr("//");

    a.eq(
        "attr.int(default=42, doc='An integer')",
        &AttrSchema::new_for_testing(
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
    let (_ctx, a) = new_assert_with_attr("//");

    let allowed = SmallSet::from_iter(vec!["foo".to_string(), "bar".to_string()]);
    a.eq(
        "attr.string(values=['foo', 'bar'], mandatory=True)",
        &AttrSchema::new_for_testing(
            AttrKind::String { allowed: Some(allowed) },
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
    let (_ctx, a) = new_assert_with_attr("//pkg");

    a.eq(
        "attr.label(allow_files=True, default=':foo')",
        &AttrSchema::new_for_testing(
            AttrKind::Label,
            Some(Attr::Label(LabelOrFile::Label(Label::new(
                PackageRef::new("//pkg").to_owned(),
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
    let (_ctx, a) = new_assert_with_attr("//");

    a.eq(
        "attr.string_list(allow_empty=False, mandatory=True)",
        &AttrSchema::new_for_testing(
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
    let (_ctx, a) = new_assert_with_attr("//");

    a.eq(
        "attr.string(default='hello')",
        &AttrSchema::new_for_testing(
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
    let (_ctx, a) = new_assert_with_attr("//");

    a.fail(
        "attr.label(allow_files=True, allow_single_file=True)",
        "allow_files and allow_single_file are mutually exclusive",
    );
}

#[test]
fn test_schema_mandatory_default_error() {
    let (_ctx, a) = new_assert_with_attr("//");

    a.fail(
        "attr.bool(mandatory=True, default=True)",
        "mandatory and default are mutually exclusive",
    );
}

#[test]
fn test_schema_default_validation_error() {
    let (_ctx, a) = new_assert_with_attr("//");

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

#[test]
fn test_attr_bool() {
    let path_resolver = PathResolver::new(std::path::PathBuf::from("/src"), "".to_string());
    let pkg = PackageRef::new("//foo");
    let heap = FrozenHeap::new();

    let schema = AttrSchema::new_for_testing(
        AttrKind::Bool,
        Some(Attr::Bool(false)),
        false,
        AllowFilesSchema::None,
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    // Test explicit true
    let attr = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc(true)))),
        pkg,
        &path_resolver,
    )
    .unwrap();
    assert_eq!(attr, Attr::Bool(true));

    // Test default when value is None
    let attr =
        Attr::create("my_attr", &schema, None, pkg, &path_resolver).unwrap();
    assert_eq!(attr, Attr::Bool(false));

    // Test non-boolean value fails
    let res = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc(42)))),
        pkg,
        &path_resolver,
    );
    assert!(res.is_err());
}

#[test]
fn test_attr_label_no_files() {
    let path_resolver = PathResolver::new(std::path::PathBuf::from("/src"), "".to_string());
    let pkg = PackageRef::new("//foo");
    let heap = FrozenHeap::new();
    let session = FakeSession::new();
    let toolchain = LabelRef::new("//".into(), "default_toolchain");

    let schema = AttrSchema::new_for_testing(
        AttrKind::Label,
        None,
        false,
        AllowFilesSchema::None,
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    // Test parsing a label ":bar"
    let attr = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc(":bar")))),
        pkg,
        &path_resolver,
    )
    .unwrap();
    let expected_label = Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned());
    assert_eq!(
        attr,
        Attr::Label(LabelOrFile::Label(expected_label.clone()))
    );
    let fake_target = FakeTarget { outputs: vec![] };
    let fake_target_ref = FakeTargetRef(std::rc::Rc::new(fake_target));
    attr.register_dependencies(&session, fake_target_ref.clone(), toolchain);
    assert_eq!(
        *session.registered_deps.lock().unwrap(),
        vec![(fake_target_ref, expected_label, session.default_toolchain.clone())]
    );

    // Test that a file string fails because files are not allowed
    let res = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc("file.cc")))),
        pkg,
        &path_resolver,
    );
    assert!(res.is_err());
}

#[test]
fn test_attr_label_allow_files() {
    let temp_dir = std::env::temp_dir().join("gn_starlark_test_attr_allow_files");
    let pkg_dir = temp_dir.join("foo");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("file.cc"), "").unwrap();

    let path_resolver = PathResolver::new(temp_dir.clone(), "".to_string());
    let pkg = PackageRef::new("//foo");
    let heap = FrozenHeap::new();
    let session = FakeSession::new();
    let toolchain = LabelRef::new("//".into(), "default_toolchain");

    let schema = AttrSchema::new_for_testing(
        AttrKind::Label,
        None,
        false,
        AllowFilesSchema::Many(AllowFiles::Some(vec!["cc".to_owned()])),
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    // Test a valid file "file.cc"
    let attr = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc("file.cc")))),
        pkg,
        &path_resolver,
    )
    .unwrap();
    let expected_path = path_resolver.source_file(pkg, "file.cc").unwrap();
    assert_eq!(attr, Attr::Label(LabelOrFile::File(expected_path)));
    
    let fake_target = FakeTarget { outputs: vec![] };
    let fake_target_ref = FakeTargetRef(std::rc::Rc::new(fake_target));
    attr.register_dependencies(&session, fake_target_ref.clone(), toolchain);
    assert!(session.registered_deps.lock().unwrap().is_empty()); // Files shouldn't register as dependencies

    // Test an invalid file "file.h" (extension not in allowed list)
    let res = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc("file.h")))),
        pkg,
        &path_resolver,
    );
    assert!(res.is_err());

    // Test that a label still resolves and registers dep
    let attr = Attr::create(
        "my_attr",
        &schema,
        Some(NoneOr::Other(Value::new_frozen(heap.alloc(":bar")))),
        pkg,
        &path_resolver,
    )
    .unwrap();
    let expected_label = Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned());
    assert_eq!(
        attr,
        Attr::Label(LabelOrFile::Label(expected_label.clone()))
    );
    
    attr.register_dependencies(&session, fake_target_ref.clone(), toolchain);
    assert_eq!(
        *session.registered_deps.lock().unwrap(),
        vec![(fake_target_ref, expected_label, session.default_toolchain.clone())]
    );

    // Clean up
    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_to_value_basic() {
    let session = FakeSession::new();
    let schema = AttrSchema::new_for_testing(
        AttrKind::Bool,
        None,
        false,
        AllowFilesSchema::None,
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    Module::with_temp_heap(|module| {
        let heap = module.heap();
        let AttrValue { attr, file, files } = Attr::Bool(true)
            .to_value(
                &schema,
                &session,
                &session.default_toolchain.as_ref(),
                &Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned())
                    .as_ref(),
                &heap,
            )
            .unwrap();

        assert!(attr.unpack_bool().unwrap());
        assert!(file.is_none());
        assert!(files.is_none());
    });
}

#[test]
fn test_to_value_label_no_files() {
    let session = FakeSession::new();
    let target_label = Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned());

    // Register the target in the fake session
    session.insert_target(
        target_label.clone(),
        FakeTargetRef(std::rc::Rc::new(FakeTarget { outputs: vec![] })),
    );

    let schema = AttrSchema::new_for_testing(
        AttrKind::Label,
        None,
        false,
        AllowFilesSchema::None,
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    Module::with_temp_heap(|module| {
        let heap = module.heap();
        let AttrValue { attr, file, files } =
            Attr::Label(LabelOrFile::Label(target_label.clone()))
                .to_value(
                    &schema,
                    &session,
                    &session.default_toolchain.as_ref(),
                    &target_label.as_ref(),
                    &heap,
                )
                .unwrap();

        // The resolved value should be the Target object
        let resolved_target = attr.downcast_ref::<FakeTargetRef>().unwrap();
        assert!(resolved_target.outputs().is_empty());
        assert!(file.is_none());
        assert!(files.is_none());
    });
}

#[test]
fn test_to_value_label_allow_files_many() {
    let session = FakeSession::new();
    let target_label = Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned());
    let label_only_file = File::from_rust("label_only.cc".to_owned());
    let overlap = File::from_rust("overlap.cc".to_owned());
    let file_only_file = File::from_rust("file_only.cc".to_owned());

    // Target outputs out.cc and overlap.h
    session.insert_target(
        target_label.clone(),
        FakeTargetRef(std::rc::Rc::new(FakeTarget {
            outputs: vec![label_only_file.clone(), overlap.clone()],
        })),
    );

    let schema = AttrSchema::new_for_testing(
        AttrKind::LabelList,
        None,
        false,
        AllowFilesSchema::Many(AllowFiles::All),
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    Module::with_temp_heap(|module| {
        let heap = module.heap();
        let AttrValue {
            attr: _,
            file,
            files,
        } = Attr::LabelList(vec![
            LabelOrFile::Label(target_label.clone()),
            LabelOrFile::File(overlap.clone()),
            LabelOrFile::File(file_only_file.clone()),
        ])
        .to_value(
            &schema,
            &session,
            &session.default_toolchain.as_ref(),
            &target_label.as_ref(),
            &heap,
        )
        .unwrap();

        assert!(file.is_none());
        // Verifying files resolved correctly: target outputs + the direct file (with overlap deduplicated!)
        assert_eq!(
            UnpackList::<&File>::unpack_value_err(files.unwrap())
                .unwrap()
                .items,
            vec![&label_only_file, &overlap, &file_only_file]
        );
    });
}

#[test]
fn test_to_value_label_allow_files_single() {
    let session = FakeSession::new();
    let target_label = Label::new(PackageRef::new("//foo").to_owned(), "bar".to_owned());
    let file1 = File::from_rust("out.cc".to_owned());

    let schema = AttrSchema::new_for_testing(
        AttrKind::Label,
        None,
        false,
        AllowFilesSchema::Single(AllowFiles::All),
        AttrCfg::CurrentToolchain,
        String::new(),
    );

    Module::with_temp_heap(|module| {
        let heap = module.heap();

        // Case 1: Target has exactly 1 output file -> succeeds
        session.insert_target(
            target_label.clone(),
            FakeTargetRef(std::rc::Rc::new(FakeTarget {
                outputs: vec![file1.clone()],
            })),
        );

        let AttrValue {
            file,
            files,
            attr: _,
        } = Attr::Label(LabelOrFile::Label(target_label.clone()))
            .to_value(
                &schema,
                &session,
                &session.default_toolchain.as_ref(),
                &target_label.as_ref(),
                &heap,
            )
            .unwrap();

        let file_val = file.unwrap();
        let single_file = file_val.downcast_ref::<File>().unwrap();
        assert_eq!(single_file, &file1);

        assert_eq!(
            UnpackList::<&File>::unpack_value_err(files.unwrap())
                .unwrap()
                .items,
            vec![&file1]
        );

        // Case 2: Direct File -> succeeds
        let AttrValue {
            file,
            files,
            attr: _,
        } = Attr::Label(LabelOrFile::File(file1.clone()))
            .to_value(
                &schema,
                &session,
                &session.default_toolchain.as_ref(),
                &target_label.as_ref(),
                &heap,
            )
            .unwrap();

        let file_val = file.unwrap();
        let single_file = file_val.downcast_ref::<File>().unwrap();
        assert_eq!(single_file, &file1);

        assert_eq!(
            UnpackList::<&File>::unpack_value_err(files.unwrap())
                .unwrap()
                .items,
            vec![&file1]
        );

        // Case 3: Target has 2 outputs -> fails
        session.insert_target(
            target_label.clone(),
            FakeTargetRef(std::rc::Rc::new(FakeTarget {
                outputs: vec![file1.clone(), File::from_rust("out.h".to_owned())],
            })),
        );

        let res = Attr::Label(LabelOrFile::Label(target_label.clone())).to_value(
            &schema,
            &session,
            &session.default_toolchain.as_ref(),
            &target_label.as_ref(),
            &heap,
        );
        assert!(res.is_err());
    });
}
