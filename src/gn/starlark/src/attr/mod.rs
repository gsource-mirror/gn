// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod schema;
pub mod cfg;
pub mod allow_files;
pub mod attr;
pub mod globals;
pub mod value;

pub use schema::{AttrSchema, FrozenAttrSchema, AttrKind, AllowFilesSchema};
pub use cfg::AttrCfg;
pub use allow_files::AllowFiles;
pub use attr::{Attr, LabelOrFile};
pub use value::AttrValue;
pub use globals::{AttrModule, register_attr};
