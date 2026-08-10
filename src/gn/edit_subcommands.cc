// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit_subcommands.h"

#include "base/containers/span.h"
#include "base/strings/string_number_conversions.h"
#include "gn/edit/build_file_resolver.h"
#include "gn/err.h"
#include "gn/input_file.h"
#include "gn/location.h"
#include "gn/parse_tree.h"
#include "gn/token.h"
#include "gn/tokenizer.h"
#include "gn/value.h"

namespace {

// Parses a single string argument into a primitive GN Value (bool, int, or
// string).
Result<Value> ParseValue(std::string_view val_string) {
  if (val_string == "true") {
    return Value(nullptr, true);
  }
  if (val_string == "false") {
    return Value(nullptr, false);
  }

  int64_t result_int;
  if (base::StringToInt64(val_string, &result_int)) {
    return Value(nullptr, result_int);
  }

  // Handle list literals like "[ a, b ]" to preserve legacy behavior.
  if (val_string.starts_with('[') && val_string.ends_with(']')) {
    InputFile file(SourceFile("//dummy"));
    file.SetContents(std::string(val_string));
    Err err;
    std::vector<Token> tokens = Tokenizer::Tokenize(&file, &err);
    if (!err.has_error() && tokens.size() >= 2 &&
        tokens.front().type() == Token::LEFT_BRACKET &&
        tokens.back().type() == Token::RIGHT_BRACKET) {
      std::vector<Value> list_elements;
      bool parsing_ok = true;
      for (size_t i = 1; i < tokens.size() - 1; ++i) {
        const Token& token = tokens[i];
        if (token.type() == Token::COMMA) {
          continue;
        }
        if (token.type() == Token::TRUE_TOKEN) {
          list_elements.push_back(Value(nullptr, true));
        } else if (token.type() == Token::FALSE_TOKEN) {
          list_elements.push_back(Value(nullptr, false));
        } else if (token.type() == Token::INTEGER) {
          int64_t val;
          if (base::StringToInt64(token.value(), &val)) {
            list_elements.push_back(Value(nullptr, val));
          } else {
            parsing_ok = false;
            break;
          }
        } else if (token.type() == Token::STRING) {
          std::string_view val = token.value();
          if (val.size() >= 2 && val.front() == '"' && val.back() == '"') {
            val.remove_prefix(1);
            val.remove_suffix(1);
          }
          list_elements.push_back(Value(nullptr, std::string(val)));
        } else {
          parsing_ok = false;
          break;
        }
      }
      if (parsing_ok) {
        Value list_val(nullptr, Value::LIST);
        list_val.list_value() = std::move(list_elements);
        return list_val;
      }
    }
  }

  return Value(nullptr, std::string(val_string));
}

// Parses multiple string arguments into a vector of GN Values.
Result<std::vector<Value>> ParseValues(base::span<const std::string> values) {
  std::vector<Value> list_elements;
  list_elements.reserve(values.size());
  for (const std::string& val_str : values) {
    ASSIGN_OR_RETURN(Value val, ParseValue(val_str));
    list_elements.push_back(std::move(val));
  }
  return list_elements;
}

}  // namespace

namespace commands {

EditCommand::~EditCommand() = default;

// Base class for edit commands that operate within a specific target.
// It automatically loops over all matched targets in a BuildFile.
class EditTargetCommand : public EditCommand {
 public:
  Err Apply(BuildFile& build_file) const override {
    for (const auto& target : build_file.targets()) {
      Err err = ApplyToTarget(build_file, target);
      if (err.has_error())
        return err;
    }
    return Ok();
  }

  // Applies this edit command to the given target in the AST.
  virtual Err ApplyToTarget(BuildFile& build_file,
                            const EditTarget& target) const = 0;
};

// Usage: set <attribute> <value>
// sets <attribute> to <value>, eg. "set testonly true".
class SetCommand : public EditTargetCommand {
 public:
  SetCommand(std::string attribute, Value value)
      : attribute_(std::move(attribute)), value_(std::move(value)) {}

  Err ApplyToTarget(BuildFile& build_file,
                    const EditTarget& target) const override {
    bool replaced = false;
    for (auto& assignment : target.assignments(attribute_)) {
      if (assignment.conditional) {
        assignment.node.add_todo(
            "This is conditional, so double check this is safe to remove");
      } else if (assignment.modification) {
        assignment.node.add_todo(
            "This is a modification, so double check this is safe to remove");
      } else {
        assignment.node->AsBinaryOpMut()->set_right(build_file.to_node(value_));
        replaced = true;
      }
    }

    if (!replaced) {
      // Construct new assignment: attribute = value_node.
      target.block->append_statement(
          build_file.create_assignment(attribute_, build_file.to_node(value_)));
    }

    return Ok();
  }

 private:
  std::string attribute_;
  Value value_;
};

Result<std::unique_ptr<EditCommand>> ParseCommand(
    std::vector<std::string> args) {
  if (args.empty()) {
    return Err(Location(), "Empty command.");
  }

  if (args[0] == "set") {
    if (args.size() < 3) {
      return Err(Location(),
                 "Invalid set command: missing attribute or value.\n"
                 "Usage: set <attribute> <value...>");
    }

    std::string_view attribute = args[1];
    bool force_list = false;
    if (attribute.ends_with(":list")) {
      attribute.remove_suffix(5);
      force_list = true;
    }

    auto value_args = base::make_span(args).subspan(2);
    Result<Value> val;
    if (value_args.size() > 1 || force_list) {
      ASSIGN_OR_RETURN(std::vector<Value> list_elements,
                       ParseValues(value_args));
      val = Value(nullptr, std::move(list_elements));
    } else {
      val = ParseValue(value_args[0]);
    }
    if (val.has_error()) {
      return val.error();
    }

    return std::unique_ptr<EditCommand>(
        std::make_unique<SetCommand>(std::string(attribute), std::move(*val)));
  }

  return Err(Location(),
             "Unknown edit command: " + std::string(args[0]) +
                 "\n"
                 "See `gn help edit` for list of supported commands.");
}

}  // namespace commands
