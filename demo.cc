#include <sstream>

struct NinjaBuf {
  // Map from variable name to value
  std::map<std::string_view, std::string> common_vars;

  std::stringbuf file_buffer;
  // When set, we are still writing the first target.
  std::optional<std::stringbuf> first_target_buffer;
  std::vector<std::string_view> current_target_vars;

  void WriteVariable(std::string_view name, std::string_view value) {
    if (first_target_buffer.has_value()) {
      common_vars[name] = value;
      Write(*first_target_buffer, key, value);
    } else if (auto it = common_vars.find(name); it != common_vars.end() && it->second != value) {
      file_buffer << "  ";
      Write(file_buffer, key, value);
      current_target_vars.push_back(key);
    }
  }

  void target_complete() {
    if (first_target_buffer.has_value()) {
      file_buffer.write(first_target_buffer);
      first_target_buffer.reset();
    } else {
      for (const auto& k : common_vars) {
        if (!current_target_vars.contains(k))
          file_buffer << "  " << k << " = " << "\n"
      }
    }
  }
};