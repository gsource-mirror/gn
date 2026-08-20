// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_file.h"

#include <sstream>

#include "util/test/test.h"

TEST(NinjaFileTest, EmptyFile) {
  NinjaFile file;
  std::ostringstream out;
  file.Serialize(out);
  EXPECT_EQ("", out.str());
}

TEST(NinjaFileTest, FileVariablesAndCustomRules) {
  NinjaFile file;
  file.file_vars.push_back({"defines", "-DFOO"});
  file.file_vars.push_back({"include_dirs", ""});
  file.custom_rules.push_back("rule custom\n  command = do_something");

  std::ostringstream out;
  file.Serialize(out);
  EXPECT_EQ(
      "defines = -DFOO\n"
      "include_dirs =\n"
      "\n"
      "rule custom\n"
      "  command = do_something\n",
      out.str());
}

TEST(NinjaFileTest, BuildEdgeFormatting) {
  NinjaFile file;
  NinjaTargetGroup group;

  NinjaBuildEdge edge;
  edge.rule = "cxx";
  edge.outputs.push_back(OutputFile("obj/foo.o"));
  edge.implicit_outputs.push_back(OutputFile("obj/foo.d"));
  edge.explicit_inputs.push_back(OutputFile("../foo.cc"));
  edge.implicit_inputs.push_back(OutputFile("obj/header.stamp"));
  edge.order_only_inputs.push_back(OutputFile("obj/order.stamp"));
  edge.validation_inputs.push_back(OutputFile("obj/validate.stamp"));
  edge.edge_vars.push_back({"source_file_part", "foo.cc"});

  group.edges.push_back(std::move(edge));
  file.AddTargetGroup(std::move(group));

  std::ostringstream out;
  file.Serialize(out);
  EXPECT_EQ(
      "build obj/foo.o | obj/foo.d: cxx ../foo.cc | obj/header.stamp || "
      "obj/order.stamp |@ obj/validate.stamp\n"
      "  source_file_part = foo.cc\n",
      out.str());
}

TEST(NinjaFileTest, HoistingSingleTarget) {
  NinjaFile file;
  NinjaTargetGroup group;
  group.target_vars.push_back({"cflags", "-fPIC"});
  group.target_vars.push_back({"defines", "-DFOO"});

  NinjaBuildEdge edge;
  edge.rule = "cxx";
  edge.outputs.push_back(OutputFile("obj/foo.o"));
  edge.explicit_inputs.push_back(OutputFile("../foo.cc"));
  edge.edge_vars.push_back({"source_file_part", "foo.cc"});
  group.edges.push_back(std::move(edge));

  file.AddTargetGroup(std::move(group));
  file.Hoist();

  std::ostringstream out;
  file.Serialize(out);
  EXPECT_EQ(
      "cflags = -fPIC\n"
      "defines = -DFOO\n"
      "\n"
      "build obj/foo.o: cxx ../foo.cc\n"
      "  source_file_part = foo.cc\n",
      out.str());
}

TEST(NinjaFileTest, HoistingMultipleTargets) {
  NinjaFile file;

  // First target group
  NinjaTargetGroup alpha;
  alpha.target_vars.push_back({"cflags", "-fPIC"});
  alpha.target_vars.push_back({"defines", "-DALPHA"});
  NinjaBuildEdge alpha_edge;
  alpha_edge.rule = "cxx";
  alpha_edge.outputs.push_back(OutputFile("obj/alpha.o"));
  alpha_edge.explicit_inputs.push_back(OutputFile("../alpha.cc"));
  alpha.edges.push_back(std::move(alpha_edge));
  file.AddTargetGroup(std::move(alpha));

  // Second target group
  NinjaTargetGroup beta;
  beta.target_vars.push_back({"cflags", "-fPIC"});
  beta.target_vars.push_back({"defines", "-DBETA"});
  NinjaBuildEdge beta_edge;
  beta_edge.rule = "cxx";
  beta_edge.outputs.push_back(OutputFile("obj/beta.o"));
  beta_edge.explicit_inputs.push_back(OutputFile("../beta.cc"));
  beta.edges.push_back(std::move(beta_edge));
  file.AddTargetGroup(std::move(beta));

  file.Hoist();

  std::ostringstream out;
  file.Serialize(out);
  EXPECT_EQ(
      "cflags = -fPIC\n"
      "\n"
      "build obj/alpha.o: cxx ../alpha.cc\n"
      "  defines = -DALPHA\n"
      "build obj/beta.o: cxx ../beta.cc\n"
      "  defines = -DBETA\n",
      out.str());
}
