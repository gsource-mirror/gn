// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#pragma once

class Err;
class Label;
class ParseNode;
class Scope;
class Settings;
class SourceDir;
class OutputFile;
class Target;
class Value;
class StarlarkValue;
class TestWithScope;
class KeyVal;
class Toolchain;

// Opaque rust types. Forward declarations only.
namespace rust {

class StarlarkSession;
class FrozenHeap;
class RustTarget;
class OwnedFrozenValue;
class StarlarkModule;
}