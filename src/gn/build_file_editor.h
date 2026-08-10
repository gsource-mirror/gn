// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_BUILD_FILE_EDITOR_H_
#define TOOLS_GN_BUILD_FILE_EDITOR_H_

#include <functional>
#include <memory>
#include <optional>
#include <unordered_map>
#include <vector>

#include "gn/err.h"
#include "gn/input_file.h"
#include "gn/label.h"
#include "gn/label_pattern.h"
#include "gn/parse_tree.h"
#include "gn/source_file.h"

class BuildSettings;
class Loader;

struct EditState;
struct EditTarget;

// A TreeNode represents a node in a tree.
// It fundamentally represents a ParseNode, but differs from one as it
// understands where it exists in the tree.
class TreeNode {
 public:
  explicit TreeNode(std::vector<ParseNode*> stack) : stack_(std::move(stack)) {
    DCHECK(!stack_.empty());
  }

  ParseNode* node() const { return stack_.back(); }
  ParseNode* parent(size_t n = 1) const {
    return stack_.size() > n ? stack_[stack_.size() - n - 1] : nullptr;
  }
  TreeNode parent_treenode(size_t n = 1) const {
    return TreeNode(std::vector<ParseNode*>(stack_.begin(), stack_.end() - n));
  }
  const std::vector<ParseNode*>& path() const { return stack_; }
  void descend(ParseNode* node) { stack_.push_back(node); }

  bool is_conditional() const;
  bool is_modification() const;

  void add_todo(EditState& state, const EditTarget& target) const;

  // Removes self from the tree, or adds a TODO suggesting that it should
  // probably be removed.
  void RemoveSelf(EditState& state, const EditTarget& target) const;

  ParseNode* operator->() const { return stack_.back(); }

 private:
  // Low-level deletion from parent block or list.
  void RemoveSelf() const;

  std::vector<ParseNode*> stack_;
};

template <typename T>
void FindNodesRecursive(
    ParseNode* node,
    std::vector<ParseNode*>& stack,
    const std::function<std::optional<T>(TreeNode&)>& transform,
    std::vector<T>* results) {
  if (!node)
    return;

  stack.push_back(node);

  TreeNode node_ref(stack);
  if (auto mapped = transform(node_ref)) {
    results->push_back(std::move(*mapped));
  }

  if (auto* block = node->AsBlock()) {
    for (const auto& stmt : block->statements()) {
      FindNodesRecursive(stmt.get(), stack, transform, results);
    }
  } else if (auto* condition = node->AsCondition()) {
    FindNodesRecursive(const_cast<BlockNode*>(condition->if_true()), stack,
                       transform, results);
    if (condition->if_false()) {
      FindNodesRecursive(const_cast<ParseNode*>(condition->if_false()), stack,
                         transform, results);
    }
  } else if (auto* func = node->AsFunctionCall()) {
    if (func->block()) {
      FindNodesRecursive(const_cast<BlockNode*>(func->block()), stack,
                         transform, results);
    }
  }

  stack.pop_back();
}

// Finds and maps nodes using the transform function starting from the root
// node.
template <typename T>
std::vector<T> FindNodes(
    ParseNode* root,
    const std::function<std::optional<T>(TreeNode&)>& transform) {
  std::vector<T> results;
  std::vector<ParseNode*> stack;
  FindNodesRecursive<T>(root, stack, transform, &results);
  return results;
}

enum MatchType {
  NONE,
  EXACT,
  GLOB,
};

class LabelMatcher {
 public:
  LabelMatcher(const SourceDir& source_dir,
               const std::vector<LabelPattern>& patterns);

  MatchType matches(const std::string& name);
  Err done() const;

 private:
  SourceDir source_dir_;
  bool globbed_ = false;
  std::unordered_map<std::string, bool> used_;
};

struct EditTarget {
  std::vector<TreeNode> assignments(std::string_view attr) const;
  std::string_view name() const { return label.name(); }
  void add_warning(EditState& state, std::string_view message) const;

  // True if the target was explicitly requested to be edited.
  // This is relevant, because if the user requests something like
  // "remove deps //dep" //:*, then we should not print warnings
  // if not all targets depend on //dep.
  // On the other hand, if the user says "remove deps //dep" //:foo,
  // and we can't find a dep on //dep, we should warn them about it.
  bool is_explicit;
  Label label;
  TreeNode node;
  BlockNode* block;
};

class BuildFile {
 public:
  static Result<BuildFile> Create(const BuildSettings* build_settings,
                                  const SourceFile& source_file,
                                  const std::vector<LabelPattern>& patterns);

  const SourceFile& source_file() const { return source_file_; }
  ParseNode* root() const { return tree_root_.get(); }
  LabelMatcher& label_matcher() { return label_matcher_; }

  // Converts a Value string into a ParseNode.
  std::unique_ptr<ParseNode> to_node(const Value& value);

  Location location() const;
  // Creates a node for an identifier.
  std::unique_ptr<IdentifierNode> create_identifier(std::string_view value);
  // Creates a node for `a = b`
  std::unique_ptr<BinaryOpNode> create_assignment(
      std::string_view name,
      std::unique_ptr<ParseNode> value);

  // Returns all targets matching the label matcher.
  std::vector<EditTarget> targets();

  // Serializes the AST to the build file if it has changed.
  // Returns Ok(true) if the file was written, Ok(false) if it was unchanged.
  Result<bool> Write();

 private:
  BuildFile(const BuildSettings* build_settings,
            SourceFile source_file,
            std::unique_ptr<InputFile> input_file,
            std::unique_ptr<ParseNode> tree_root,
            LabelMatcher label_matcher);

  const BuildSettings* build_settings_;
  SourceFile source_file_;
  std::unique_ptr<InputFile> input_file_;
  std::unique_ptr<ParseNode> tree_root_;
  std::vector<std::unique_ptr<InputFile>> extra_files_;
  LabelMatcher label_matcher_;
};

// Resolves a list of LabelPatterns into unique parsed BuildFiles.
Result<std::vector<BuildFile>> ResolvePatternsToBuildFiles(
    const BuildSettings* build_settings,
    const Loader* loader,
    const std::vector<LabelPattern>& patterns);

#endif  // TOOLS_GN_BUILD_FILE_EDITOR_H_
