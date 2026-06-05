// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::Path;

use starlark::values::list::UnpackList;

use crate::ctx::gn::Gn;
use crate::Assert;
use crate::File;
use crate::LabelRef;

#[test]
fn test_gn_get_output_files() {
    let mut a = Assert::new();
    let label = LabelRef::new("//".into(), "source_set");

    a.globals_add(move |builder| {
        let gn_gen = Gn::new();
        builder.set("my_gn", gn_gen);
    });
    a.rule_eval(label, |a| {
        // For performance reasons, File objects do pointer based equality checks, so we can't directly do a.eq
        let sources: UnpackList<&File> = a.eval("my_gn.sources()");
        assert_eq!(sources.items.len(), 1);
        assert_eq!(sources.items[0].as_path(), Path::new("../../source_set.cc"));

        let public: UnpackList<&File> = a.eval("my_gn.public()");
        assert_eq!(public.items.len(), 1);
        assert_eq!(public.items[0].as_path(), Path::new("../../source_set.h"));
    })
}
