(ns statusline-builder.segment
  (:require [statusline-builder.colour :as colour]))

(def presets
  {"Model" {"color" (colour/rgb 203 166 247)}
   "Effort" {"color" (colour/grey)}
   "ContextUsage" {"color" colour/gradient
                   "prefix_color" (colour/rgb 203 166 247)
                   "suffix_color" (colour/rgb 203 166 247)}
   "PromptCacheTtl" {"color" colour/gradient "prefix" "cache "}
   "RateLimit" {"style" "Percent"
                "fill" "Remaining"
                "color_mode" "Gradient"
                "gradient_midpoint_percentage" 50
                "usage_ttl_seconds" 300
                "low_color" (colour/rgb 166 227 161)
                "mid_color" (colour/rgb 249 226 175)
                "high_color" (colour/rgb 243 139 168)}
   "SubagentStats" {"color" (colour/grey)
                    "active_color" (colour/rgb 166 227 161)
                    "stall_color" (colour/rgb 243 139 168)
                    "prefix" "Sub-agents:"
                    "stall_marker" "!"
                    "stall_seconds" 120
                    "show_tokens" true}
   "TokensByModel" {"color" (colour/grey) "prefix" "tokens " "separator" " · "}
   "TokenSpend" {"color" (colour/rgb 205 214 244) "label_color" (colour/rgb 127 132 156)}
   "SessionCost" {"color" (colour/rgb 249 226 175) "prefix" ""}
   "Reminder" {"color" (colour/rgb 250 179 135) "prefix" "⏰ " "separator" " · " "max_chars" 80}
   "SessionNotice" {"color" (colour/rgb 249 226 175)
                    "prefix" "📌 "
                    "max_chars" 120
                    "show_remaining" true
                    "standalone" true}
   "Cwd" {"color" (colour/rgb 137 180 250)}
   "GitBranch" {"color" (colour/rgb 148 226 213)
                "state_color" (colour/rgb 250 179 135)
                "show_worktree" true
                "show_ahead_behind" true
                "show_state" true}
   "GitDiff" {"modified_color" (colour/rgb 116 199 236)
              "untracked_color" (colour/rgb 116 199 236)
              "deleted_color" (colour/rgb 243 139 168)}
   "GitError" {"color" (colour/rgb 243 139 168) "text" "no git"}
   "MyLastPrompt" {"color" (colour/grey) "prefix" "» " "max_chars" 48 "standalone" true}
   "ClaudeResourceUsage" {"color" (colour/grey) "cpu_prefix" "CPU " "memory_prefix" "RSS "}
   "UserIdleTime" {"color" (colour/grey) "prefix" "idle "}
   "Spacer" {"shape" "LineBreak"}
   "CommandOutput" {"color" (colour/grey)
                    "prefix" "ping "
                    "command" "ping"
                    "args" ["-c" "1" "8.8.8.8"]
                    "prompt" ""
                    "ttl_seconds" 60
                    "max_chars" 80}
   "LlmInsight" {"color" (colour/rgb 186 194 222)
                 "prefix" "🎯 "
                 "command" "codex"
                 "args" ["exec" "--skip-git-repo-check" "-s" "read-only" "-c" "approval_policy=never"]
                 "prompt" "In one sentence: what the user's goal is and what is being done for it right now. Only the sentence, no quotes or explanations."
                 "every_turns" 2
                 "scan_whole_session" true
                 "initial_scan_kib" 256
                 "context_chars" 12000
                 "max_chars" 128
                 "standalone" true}
   "Weather" {"color" (colour/grey) "format" "%c+%t" "ttl_seconds" 1800 "max_chars" 32}})

(def default-line
  ["Model"
   "RateLimit:Fable"
   "ContextUsage"
   "Effort"
   "PromptCacheTtl"
   "RateLimit:FiveHour"
   "RateLimit:SevenDay"
   "SubagentStats"
   "TokensByModel"
   "Cwd"
   "GitBranch"
   "GitDiff"
   "MyLastPrompt"
   "GitError"
   "LlmInsight"
   {:module "LlmInsight"
    :config {"color" (colour/rgb 245 224 220)
             "prefix" "💡 "
             "prompt" "In one sentence: what should be added to the user's last prompt to make the task more precise. Do not suggest what is already done. Only the suggestion, no quotes or explanations."}}])

(def ^:private always-standalone #{"CommandOutput"})

(def ^:private truncating #{"MyLastPrompt"})

(defn standalone? [segment]
  (or (contains? always-standalone (:type segment))
      (true? (get-in segment [:config "standalone"]))
      (= "BlankLine" (get-in segment [:config "shape"]))))

(defn line-break? [segment]
  (= "LineBreak" (get-in segment [:config "shape"])))

(defn divider? [segment]
  (= "Divider" (get-in segment [:config "shape"])))

(defn overflow [segment]
  (if (contains? truncating (:type segment)) :truncate :wrap))

(defn- field-default [{:keys [kind variants]}]
  (case kind
    "color" (colour/grey)
    "text" ""
    "bool" false
    ("integer" "float") 0
    ("list" "pairs") []
    "enum" (first variants)
    (throw (ex-info (str "Unknown field kind: " kind) {}))))

(defn- defaults [module]
  (let [preset (get presets (:type module) {})]
    (merge (into {} (map (fn [field]
                           [(:name field) (if (contains? preset (:name field))
                                            (get preset (:name field))
                                            (field-default field))]))
                 (:fields module))
           (:overrides module))))

(defn create [module]
  {:id (str "segment-" (random-uuid))
   :module-id (:id module)
   :type (:type module)
   :config (defaults module)})
