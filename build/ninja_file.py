# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

"""A Python DSL for generating Ninja build files."""

import dataclasses
import os
import pathlib
import sys

def chain(*commands):
  return ' && '.join(commands)

@dataclasses.dataclass
class Action:
  rule: 'DummyRule'
  output: pathlib.Path
  inputs: list[pathlib.Path] = dataclasses.field(default_factory=list)
  implicit_inputs: list[pathlib.Path] = dataclasses.field(default_factory=list)
  variables: dict[str, str] = dataclasses.field(default_factory=dict)

  def __str__(self):
    return str(self.output)

class DummyRule:
  def __init__(self, name, ninja_file, inputs=None):
    self.name = name
    self.ninja_file = ninja_file
    self.inputs = inputs or []

  def __call__(self, output, *, inputs=[], implicit_inputs=[], **kwargs):
    action = Action(
        rule=self,
        output=pathlib.Path(output),
        inputs=inputs,
        implicit_inputs=self.inputs + implicit_inputs,
        variables=kwargs
    )
    self.ninja_file.actions.append(action)
    return action.output

class Rule(DummyRule):
  def __init__(self, name, ninja_file, command, description=None, inputs=None):
    super().__init__(name, ninja_file, inputs)
    self.command = command
    self.description = description
    self.ninja_file.rules.append(self)

class NinjaFile:
  def __init__(self, platform, source_root, out_dir):
    self.platform = platform
    self.out_dir = pathlib.Path(out_dir).resolve()
    # source_root is relative to out_dir
    self.source_root = pathlib.Path(os.path.relpath(source_root, self.out_dir))

    self.regen_triggers = []
    self.rules = []
    self.actions = []
    
    self._gn_exe = pathlib.Path('gn' + self.platform.exe_suffix)

    def python(path, args):
      return f'{sys.executable} {self.source_file(path)} {args}'

    # Define standard/dummy rules (no rule block generated in build.ninja)
    self.Phony = DummyRule('phony', self)
    self.Cxx = DummyRule('cxx', self)
    self.Link = DummyRule('link', self)

    self.RunBinary = Rule(
        name='run_binary',
        ninja_file=self,
        command=chain(
            '$in $args',
            python('tools/touch.py', '$out')
        ),
        description='RUN BINARY $in'
    )

    # Define custom rules (rule block generated in build.ninja)
    self._run_gn = Rule(
        name='run_gn',
        ninja_file=self,
        command=chain(
            # For golden tests it's very important that if a ninja file is no
            # longer generated, it is actually deleted.
            python('tools/clean.py', '$out.actual'),
            f'{self._gn_exe} gen $out.actual --root=$path',
            python('tools/touch.py', '$out')
        ),
        description='RUN GN $out',
        inputs=[self._gn_exe]
    )

    compare_script = self.source_file('tools/compare_goldens.py')
    self._compare_goldens = Rule(
        name='compare_goldens',
        ninja_file=self,
        command=chain(
            python('tools/compare_goldens.py', '$path $goldens'),
            python('tools/touch.py', '$out')
        ),
        description='COMPARE $out',
        inputs=[compare_script]
    )

  def source_file(self, path):
    return self.source_root / path

  def directory(self, dir_path, exclude_dirs):
    # Join out_dir with dir_path (which is relative to out_dir) to get absolute path for filesystem walk
    full_dir_path = (self.out_dir / dir_path).resolve()
    inputs = []
    for root, dirs, files in os.walk(full_dir_path):
      for exc in exclude_dirs:
        if exc in dirs:
          dirs.remove(exc)
      root = pathlib.Path(root)
      self.regen_triggers.append(root)
      inputs.append(root)
      for file in files:
        full_path = pathlib.Path(root) / file
        rel_path = pathlib.Path(os.path.relpath(full_path, self.out_dir))
        inputs.append(rel_path)
    return sorted(inputs)
  
  # IntegrationTest is a macro more so than a rule.
  def IntegrationTest(self, name):
    path = self.source_file(f'integration_tests/{name}')
    inputs = self.directory(path, ['out', 'goldens'])
    golden_path = path / 'goldens'
    
    stamp = self._run_gn(name, path=path, inputs=inputs)
    
    return self._compare_goldens(
        name + "_integration_test",
        inputs=[stamp] + self.directory(golden_path, []),
        path=f'{name}.actual',
        goldens=golden_path
    )
    
  def write_ninja(self):
    out = []
    for rule in self.rules:
      out.append(f'rule {rule.name}')
      out.append(f'  command = {getattr(rule, "command", "")}')
      out.append(f'  description = {getattr(rule, "description", "")}')
      out.append('')
    
    for action in self.actions:
      inputs = ' '.join(map(str, action.inputs))
      inputs = f' {inputs}' if inputs else ''
      
      implicit = ' '.join(map(str, action.implicit_inputs))
      implicit = f' | {implicit}' if implicit else ''
      
      out.append(f'build {action.output}: {action.rule.name}{inputs}{implicit}')
      for k, v in action.variables.items():
        out.append(f'  {k} = {v}')
      out.append('')
    
    return '\n'.join(out)