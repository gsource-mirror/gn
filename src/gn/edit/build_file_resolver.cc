// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit/build_file_resolver.h"

#include "base/files/file_enumerator.h"
#include "base/files/file_util.h"
#include "gn/build_settings.h"
#include "gn/filesystem_utils.h"
#include "gn/input_file.h"
#include "gn/label.h"
#include "gn/loader.h"
#include "gn/parse_tree.h"
#include "gn/parser.h"
#include "gn/string_atom.h"
#include "gn/tokenizer.h"

namespace commands {

namespace {

std::optional<std::string> AsStringLiteral(const ParseNode* node) {
  auto* literal = node->AsLiteral();
  if (!literal || literal->value().type() != Token::STRING) {
    return std::nullopt;
  }
  // Note: This is very rudimentary and does not escape special characters.
  // We don't really care though since it's unlikely to have special characters
  // in target names.
  std::string_view val = literal->value().value();
  DCHECK(val.size() >= 2 && val.front() == '"' && val.back() == '"');
  return std::string(val.substr(1, val.size() - 2));
}

// Resolves a single LabelPattern to matching SourceFiles.
Result<std::vector<SourceFile>> ResolvePatternToFiles(
    const BuildSettings* build_settings,
    const Loader* loader,
    const LabelPattern& pattern) {
  std::vector<SourceFile> matched_files;
  auto add_dir = [&](const SourceDir& dir) {
    auto build_file = loader->BuildFileForLabel(Label(dir, "dummy"));
    if (base::PathExists(build_settings->GetFullPath(build_file))) {
      matched_files.push_back(build_file);
    }
  };

  add_dir(pattern.dir());

  if (pattern.type() == LabelPattern::MATCH ||
      pattern.type() == LabelPattern::DIRECTORY) {
    if (matched_files.empty()) {
      return Err(
          Location(),
          "Build file does not exist: " +
              loader->BuildFileForLabel(Label(pattern.dir(), "dummy")).value());
    }
    return matched_files;
  }

  base::FilePath disk_path = build_settings->GetFullPath(pattern.dir());
  if (!base::DirectoryExists(disk_path)) {
    return Err(Location(),
               "Directory does not exist: " + pattern.dir().value());
  }

  base::FileEnumerator traverser(disk_path, /*recursive=*/true,
                                 base::FileEnumerator::DIRECTORIES);
  for (base::FilePath current = traverser.Next(); !current.empty();
       current = traverser.Next()) {
    base::FilePath relative;
    if (build_settings->root_path().AppendRelativePath(current, &relative)) {
      std::string source_path = "//" + FilePathToUTF8(relative) + "/";
      NormalizePath(&source_path);
      add_dir(SourceDir(source_path));
    }
  }

  return matched_files;
}

}  // namespace

Result<BuildFile> BuildFile::Create(const BuildSettings* build_settings,
                                    const SourceFile& source_file,
                                    const std::vector<LabelPattern>& patterns) {
  auto input_file = std::make_unique<InputFile>(source_file);

  base::FilePath full_path = build_settings->GetFullPath(source_file);

  if (!input_file->Load(full_path)) {
    return Err(Location(), "Could not load file: " + source_file.value());
  }

  Err err;
  std::vector<Token> tokens = Tokenizer::Tokenize(input_file.get(), &err);
  if (err.has_error()) {
    return err;
  }

  std::unique_ptr<ParseNode> tree_root = Parser::Parse(tokens, &err);
  if (err.has_error()) {
    return err;
  }

  LabelMatcher label_matcher(source_file.GetDir(), patterns);
  return BuildFile(source_file, std::move(input_file), std::move(tree_root),
                   std::move(label_matcher));
}

BuildFile::BuildFile(SourceFile source_file,
                     std::unique_ptr<InputFile> input_file,
                     std::unique_ptr<ParseNode> tree_root,
                     LabelMatcher label_matcher)
    : source_file_(std::move(source_file)),
      input_file_(std::move(input_file)),
      tree_root_(std::move(tree_root)),
      label_matcher_(std::move(label_matcher)) {}

LabelMatcher::LabelMatcher(const SourceDir& source_dir,
                           const std::vector<LabelPattern>& patterns)
    : source_dir_(source_dir), globbed_(false) {
  for (const auto& pattern : patterns) {
    if (pattern.type() == LabelPattern::RECURSIVE_DIRECTORY &&
        source_dir.value().starts_with(pattern.dir().value())) {
      globbed_ = true;
    } else if (pattern.type() == LabelPattern::DIRECTORY &&
               source_dir == pattern.dir()) {
      globbed_ = true;
    } else if (pattern.type() == LabelPattern::MATCH &&
               pattern.dir() == source_dir) {
      used_[pattern.name()] = false;
    }
  }
}

bool LabelMatcher::matches(const std::string& name) {
  if (auto it = used_.find(name); it != used_.end()) {
    it->second = true;  // Mark as used.
    return true;
  }
  return globbed_;
}

Err LabelMatcher::done() const {
  std::vector<std::string> unused;
  for (const auto& [name, used] : used_) {
    if (!used) {
      unused.push_back(name);
    }
  }
  if (!unused.empty()) {
    std::sort(unused.begin(), unused.end());
    std::string msg = "Target(s) not found: ";
    for (size_t i = 0; i < unused.size(); ++i) {
      if (i > 0)
        msg += ", ";
      msg += Label(source_dir_, unused[i]).GetUserVisibleName(false);
    }
    return Err(Location(), msg);
  }
  return Ok();
}

Result<std::vector<BuildFile>> ResolvePatternsToBuildFiles(
    const BuildSettings* build_settings,
    const Loader* loader,
    const std::vector<LabelPattern>& patterns) {
  std::set<SourceFile> seen;
  std::vector<BuildFile> result;

  for (const LabelPattern& pattern : patterns) {
    ASSIGN_OR_RETURN(auto files,
                     ResolvePatternToFiles(build_settings, loader, pattern));

    for (const SourceFile& file : files) {
      if (seen.insert(file).second) {
        ASSIGN_OR_RETURN(auto parsed,
                         BuildFile::Create(build_settings, file, patterns));
        result.push_back(std::move(parsed));
      }
    }
  }
  return result;
}

Location BuildFile::location() const {
  return Location(input_file_.get(), 0, 0);
}

std::unique_ptr<ParseNode> BuildFile::parse(std::string_view value) {
  auto file = std::make_unique<InputFile>(SourceFile("//dummy"));
  file->SetContents(std::string(value));

  Err err;
  std::vector<Token> tokens = Tokenizer::Tokenize(file.get(), &err);
  if (!err.has_error()) {
    // Since generated tokens don't have a location in the source, we create an
    // empty location.
    for (auto& token : tokens) {
      token.set_location(this->location());
    }
    auto parsed = Parser::ParseExpression(tokens, &err);
    // The spec of this function requires an identifier (eg. `abc`) to be
    // interpreted as the string literal "abc".
    if (!err.has_error() && !parsed->AsIdentifier()) {
      // The tokens contain a string_view pointing to InputFile's buffer.
      // So we take ownership of the buffer to ensure we don't use-after-free.
      extra_files_.push_back(std::move(file));
      return parsed;
    }
  }

  // Fallback for unquoted strings:
  std::string quoted = "\"" + std::string(value) + "\"";
  // Intern it so that LiteralNode's string_view lives long enough.
  StringAtom atom(quoted);

  return std::make_unique<LiteralNode>(
      Token(location(), Token::STRING, atom.str()));
}

std::vector<EditTarget> BuildFile::targets() {
  return FindNodes<EditTarget>(
      tree_root_.get(),
      [this](TreeNode& node_ref) -> std::optional<EditTarget> {
        if (auto* func = node_ref->AsFunctionCallMut()) {
          if (func->block() && func->args() &&
              func->args()->contents().size() == 1) {
            if (auto name =
                    AsStringLiteral(func->args()->contents()[0].get())) {
              if (label_matcher_.matches(*name)) {
                return EditTarget{
                    .name = *name,
                    .node = node_ref,
                    .block = func->block(),
                };
              }
            }
          }
        }
        return std::nullopt;
      });
}

std::vector<Assignment> EditTarget::assignments(std::string_view attr) const {
  return FindNodes<Assignment>(
      block, [attr](TreeNode& node_ref) -> std::optional<Assignment> {
        if (auto* op = node_ref->AsBinaryOp()) {
          if (op->op().type() == Token::EQUAL ||
              op->op().type() == Token::PLUS_EQUALS ||
              op->op().type() == Token::MINUS_EQUALS) {
            if (auto* left = op->left()->AsIdentifier()) {
              if (left->value().value() == attr) {
                bool conditional = false;
                for (const auto* ancestor : node_ref.path()) {
                  if (ancestor->AsCondition()) {
                    conditional = true;
                    break;
                  }
                  // We don't care if the target as a whole is conditional.
                  if (ancestor->AsFunctionCall()) {
                    break;
                  }
                }
                return Assignment{
                    .modification = (op->op().type() != Token::EQUAL),
                    .conditional = conditional,
                    .node = node_ref};
              }
            }
          }
        }
        return std::nullopt;
      });
}

std::unique_ptr<IdentifierNode> BuildFile::create_identifier(
    std::string_view value) {
  StringAtom atom(value);
  return std::make_unique<IdentifierNode>(
      Token(location(), Token::IDENTIFIER, atom.str()));
}

std::unique_ptr<BinaryOpNode> BuildFile::create_assignment(
    std::string_view name,
    std::unique_ptr<ParseNode> value) {
  auto left = create_identifier(name);

  auto assign = std::make_unique<BinaryOpNode>();
  assign->set_op(Token(location(), Token::EQUAL, "="));
  assign->set_left(std::move(left));
  assign->set_right(std::move(value));

  return assign;
}

void TreeNode::add_todo(std::string_view message) const {
  std::string comment_text = "# TODO(gn edit): " + std::string(message);
  StringAtom atom(comment_text);
  Token comment_token(node()->GetRange().begin(), Token::LINE_COMMENT,
                      atom.str());
  node()->comments_mutable()->append_before(std::move(comment_token));
}

}  // namespace commands
