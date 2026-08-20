// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_file.h"

#include <algorithm>
#include <map>
#include <ostream>
#include <set>
#include <string>

#include "gn/escape.h"
#include "gn/string_output_buffer.h"

namespace {

void WriteOutputFile(std::ostream& out, const OutputFile& file) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA;
  out << " ";
  EscapeStringToStream(out, file.value(), opts);
}

void WriteVariable(std::ostream& out,
                   std::string_view name,
                   const std::string& value,
                   bool indent = false) {
  if (indent)
    out << "  ";
  out << name << " =";
  if (!value.empty()) {
    if (!value.starts_with(' '))
      out << " ";
    out << value;
  }
  out << "\n";
}

}  // namespace

void NinjaFile::AddTargetGroup(NinjaTargetGroup group) {
  targets.push_back(std::move(group));
}

void NinjaFile::Hoist() {
  if (targets.empty())
    return;

  if (targets.size() == 1) {
    for (auto& var : targets.front().target_vars) {
      file_vars.push_back(std::move(var));
    }
    targets.front().target_vars.clear();
    for (auto& rule : targets.front().custom_rules) {
      custom_rules.push_back(std::move(rule));
    }
    targets.front().custom_rules.clear();
    return;
  }

  struct Candidate {
    std::string value;
    bool is_uniform = true;
    size_t count = 0;
  };
  std::map<std::string_view, Candidate> candidates;

  for (const auto& target_group : targets) {
    for (const auto& var : target_group.target_vars) {
      if (var.value.empty())
        continue;
      auto& cand = candidates[var.name];
      if (cand.count == 0) {
        cand.value = var.value;
      } else if (cand.value != var.value) {
        cand.is_uniform = false;
      }
      cand.count++;
    }
  }

  std::set<std::string_view> hoisted_names;
  for (const auto& [name, cand] : candidates) {
    if (cand.is_uniform && cand.count == targets.size()) {
      file_vars.push_back({name, cand.value});
      hoisted_names.insert(name);
    }
  }

  for (auto& target_group : targets) {
    std::erase_if(target_group.target_vars, [&](const NinjaVariable& var) {
      return hoisted_names.contains(var.name);
    });
  }
}

void NinjaFile::Serialize(std::ostream& out) const {
  for (const auto& var : file_vars) {
    WriteVariable(out, var.name, var.value, /*indent=*/false);
  }
  if (!file_vars.empty())
    out << "\n";

  for (const auto& rule : custom_rules) {
    out << rule;
    if (!rule.ends_with("\n"))
      out << "\n";
  }

  for (size_t t = 0; t < targets.size(); ++t) {
    const auto& target_group = targets[t];
    for (const auto& rule : target_group.custom_rules) {
      out << rule;
      if (!rule.ends_with("\n"))
        out << "\n";
    }
    for (size_t e = 0; e < target_group.edges.size(); ++e) {
      const auto& edge = target_group.edges[e];
      if (edge.blank_line_before)
        out << "\n";
      out << "build";
      for (const auto& output : edge.outputs)
        WriteOutputFile(out, output);
      if (!edge.implicit_outputs.empty()) {
        out << " |";
        for (const auto& out_file : edge.implicit_outputs)
          WriteOutputFile(out, out_file);
      }
      out << ": " << edge.rule;
      for (const auto& input : edge.explicit_inputs)
        WriteOutputFile(out, input);
      if (!edge.implicit_inputs.empty()) {
        out << " |";
        for (const auto& in_file : edge.implicit_inputs)
          WriteOutputFile(out, in_file);
      }
      if (!edge.order_only_inputs.empty()) {
        out << " ||";
        for (const auto& in_file : edge.order_only_inputs)
          WriteOutputFile(out, in_file);
      }
      if (!edge.validation_inputs.empty()) {
        out << " |@";
        for (const auto& in_file : edge.validation_inputs)
          WriteOutputFile(out, in_file);
      }
      out << "\n";

      for (const auto& var : target_group.target_vars) {
        WriteVariable(out, var.name, var.value, /*indent=*/true);
      }
      for (const auto& var : edge.edge_vars) {
        WriteVariable(out, var.name, var.value, /*indent=*/true);
      }
    }
  }
}

void NinjaFile::Serialize(StringOutputBuffer& out) const {
  std::ostream os(&out);
  Serialize(os);
}
