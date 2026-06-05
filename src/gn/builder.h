// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_BUILDER_H_
#define TOOLS_GN_BUILDER_H_

#include <functional>
#include <memory>
#include <unordered_map>

#include "gn/builder_record.h"
#include "gn/builder_record_map.h"
#include "gn/label.h"
#include "gn/label_ptr.h"
#include "gn/unique_vector.h"

class ActionValues;
class Err;
class Loader;
class ParseNode;
class Target;

// The builder assembles the dependency tree. It is not threadsafe and runs on
// the main thread only. See also BuilderRecord.
class Builder {
 public:
  using ResolvedGeneratedCallback = std::function<void(const BuilderRecord*)>;

  // A callback invoked once the graph has been fully resolved to
  // write generated_file() outputs to disk. This is only possible once
  // all targets have been resolved, to be able to perform metadata walks
  // safely, even over transitive validation dependencies.
  using WriteGeneratedFilesCallback =
      std::function<void(std::vector<const Target*>& generated_file_targets)>;

  explicit Builder(Loader* loader);
  ~Builder();

  // The resolved callback is called when a target has been both resolved and
  // marked generated. This will be executed only on the main thread.
  void set_resolved_and_generated_callback(
      const ResolvedGeneratedCallback& cb) {
    resolved_and_generated_callback_ = cb;
  }

  // The completion callback is called when all items in the graph have been
  // resolved. This allows performing metadata walks safely, even over
  // transitive validation deps.
  void set_write_generated_files_callback(
      const WriteGeneratedFilesCallback& cb) {
    write_generated_files_callback_ = cb;
  }

  Loader* loader() const { return loader_; }

  void ItemDefined(std::unique_ptr<Item> item);

  // Returns NULL if there is not a thing with the corresponding label.
  const Item* GetItem(const Label& label) const;
  const Toolchain* GetToolchain(const Label& label) const;

  std::vector<const BuilderRecord*> GetAllRecords() const;

  // Returns items which should be generated and which are defined.
  std::vector<const Item*> GetAllResolvedItems() const;

  // Returns targets which should be generated and which are defined.
  std::vector<const Target*> GetAllResolvedTargets() const;

  // Returns the record for the given label, or NULL if it doesn't exist.
  // Mostly used for unit tests.
  const BuilderRecord* GetRecord(const Label& label) const;
  BuilderRecord* GetRecord(const Label& label);

  // If there are any undefined references, returns false and sets the error.
  bool CheckForBadItems(Err* err) const;

  // Get or create an empty record for unit-testing.
  BuilderRecord* GetOrCreateRecordForTesting(const Label& label);

  // Invoke the WriteGeneratedFilesCallback if there are generated_file()
  // targets in the graph. This must be called *after* all items are
  // resolved to ensure metadata walks can be performed safely.
  void WriteGeneratedFilesAfterFullResolution();

 private:
  bool TargetDefined(BuilderRecord* record, Err* err);
  bool ConfigDefined(BuilderRecord* record, Err* err);
  bool ToolchainDefined(BuilderRecord* record, Err* err);

  // Returns the record associated with the given label. This function checks
  // that if we already have references for it, the type matches. If no record
  // exists yet, a new one will be created.
  //
  // If any of the conditions fail, the return value will be null and the error
  // will be set. request_from is used as the source of the error.
  BuilderRecord* GetOrCreateRecordOfType(const Label& label,
                                         const ParseNode* request_from,
                                         BuilderRecord::ItemType type,
                                         Err* err);

  // Returns the record associated with the given label. This function checks
  // that it's already been resolved to the correct type.
  //
  // If any of the conditions fail, the return value will be null and the error
  // will be set. request_from is used as the source of the error.
  BuilderRecord* GetResolvedRecordOfType(const Label& label,
                                         const ParseNode* request_from,
                                         BuilderRecord::ItemType type,
                                         Err* err);

  bool AddDeps(BuilderRecord* record,
               const LabelConfigVector& configs,
               Err* err);
  bool AddDeps(BuilderRecord* record,
               const UniqueVector<LabelConfigPair>& configs,
               Err* err);
  bool AddDeps(BuilderRecord* record,
               const LabelTargetVector& targets,
               Err* err);
  bool AddGenDeps(BuilderRecord* record,
                  const LabelTargetVector& targets,
                  Err* err);
  bool AddPoolDep(BuilderRecord* record, const Target* target, Err* err);
  bool AddToolchainDep(BuilderRecord* record, const Target* target, Err* err);
  bool AddValidationDeps(BuilderRecord* record,
                         const LabelTargetVector& targets,
                         Err* err);

  // Given a target, sets the "should generate" bit and pushes it through the
  // dependency tree. Any time the bit it set, we ensure that the given item is
  // scheduled to be loaded.
  //
  // If the force flag is set, we'll ignore the current state of the record's
  // should_generate flag, and set it on the dependents every time. This is
  // used when defining a target: the "should generate" may have been set
  // before the item was defined (if it is required by something that is
  // required). In this case, we need to re-push the "should generate" flag
  // to the item's dependencies.
  void RecursiveSetShouldGenerate(BuilderRecord* record, bool force);

  void ScheduleItemLoadIfNecessary(BuilderRecord* record);

  // Update the state of a BuilderRecord and triggers subsequent steps
  // like dependency resolution or finalization based on the state change.
  // |prev_state| is the state of the record before the update.
  bool UpdateItem(BuilderRecord* record,
                  BuilderRecord::RecordState prev_state,
                  Err* err);

  // This takes a BuilderRecord with resolved dependencies, and fills in the
  // target's Label*Vectors with the resolved pointers.
  bool ResolveItem(BuilderRecord* record, Err* err);
  void ScheduleBackgroundTargetChecks(BuilderRecord* record);
  bool CompleteItemResolution(BuilderRecord* record, Err* err);

  // Finalizes the given item, scheduling its write to the Ninja file if
  // should_generate() is true, then notifying all dependents.
  bool FinalizeItem(BuilderRecord* record, Err* err);

  // Fills in the pointers in the given vector based on the labels. We assume
  // that everything should be resolved by this point, so will return an error
  // if anything isn't found or if the type doesn't match.
  bool ResolveDeps(LabelTargetVector* deps, Err* err);
  bool ResolveValidationDeps(LabelTargetVector* deps, Err* err);
  bool ResolveConfigs(UniqueVector<LabelConfigPair>* configs, Err* err);
  bool ResolvePool(Target* target, Err* err);
  bool ResolveToolchain(Target* target, Err* err);
  bool ResolvePools(Toolchain* toolchain, Err* err);

  // Checks if the given record is ready to be written (resolved, generated,
  // and writable) and calls the callback if so.
  void CheckAndTriggerWrite(BuilderRecord* record);

  // Given a list of unresolved records, tries to find any circular
  // dependencies and returns the string describing the problem. If no circular
  // deps were found, returns the empty string.
  std::string CheckForCircularDependencies(
      const std::vector<const BuilderRecord*>& bad_records) const;

  // Non owning pointer.
  Loader* loader_;

  BuilderRecordMap records_;

  ResolvedGeneratedCallback resolved_and_generated_callback_;

  // The generated_file() targets found during resolution.
  std::vector<const Target*> generated_file_targets_;
  WriteGeneratedFilesCallback write_generated_files_callback_;

  Builder(const Builder&) = delete;
  Builder& operator=(const Builder&) = delete;
};

#endif  // TOOLS_GN_BUILDER_H_
