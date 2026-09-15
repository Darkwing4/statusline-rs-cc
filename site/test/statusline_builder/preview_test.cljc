(ns statusline-builder.preview-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.fixture :as fixture]
            [statusline-builder.preview :as preview]
            [statusline-builder.scenarios :as scenarios]
            [statusline-builder.width :as width]))

(defn- text-of [segment session]
  (:text (first (preview/pieces segment session))))

(deftest every-piece-draws-something-in-at-least-one-session
  (doseq [module fixture/modules]
    (let [segment (fixture/create (:id module))]
      (is (some #(seq (preview/pieces segment %)) scenarios/all) (:id module)))))

(deftest model-replacements-are-applied-in-order-in-the-preview
  (let [model (fixture/create "Model")]
    (is (= "Opus" (text-of (fixture/with-field model "replacements" [[" (1M context)" ""] ["Opus 5" "Opus"]]) (first scenarios/all))))
    (is (nil? (preview/pieces (fixture/with-field model "replacements" [["Opus 5 (1M context)" ""]]) (first scenarios/all))))))

(deftest the-fable-window-shows-its-markers-only-in-a-fable-session
  (let [fable (-> (fixture/create "RateLimit:Fable")
                  (fixture/with-field "prefix" "{t}d ")
                  (fixture/with-field "active_marker" "*")
                  (fixture/with-field "severity_markers" [["warning" "!"]]))]
    (is (nil? (preview/pieces fable (first scenarios/all))))
    (is (= "1.5d 76%!*" (text-of fable (fixture/scenario "fable"))))))

(deftest subagent-stats-follow-the-runtime-format
  (let [stats (fixture/create "SubagentStats")]
    (is (= "Sub-agents:2/7 4m12s 1.2M" (text-of stats (fixture/scenario "active"))))
    (is (= "Sub-agents:7 42k" (text-of stats (fixture/scenario "clean"))))
    (is (= "Sub-agents:1/3! 1h02m" (text-of stats (fixture/scenario "pressure"))))
    (is (nil? (preview/pieces stats (fixture/scenario "outside"))))))

(deftest tokens-by-model-follow-the-runtime-format
  (let [tokens (fixture/create "TokensByModel")
        clean (fixture/scenario "clean")]
    (is (= "tokens opus-5 3.4M · haiku-4-5 45k" (text-of tokens (fixture/scenario "active"))))
    (is (= "tokens sonnet-5 640k · haiku-4-5 42k" (text-of tokens (assoc clean :model-tokens [["haiku-4-5" 42000] ["opus-5" 0] ["sonnet-5" 640000]]))))
    (is (nil? (preview/pieces tokens (assoc clean :model-tokens []))))))

(deftest token-spend-follows-the-runtime-format
  (let [turn (fixture/create "TokenSpend")
        session (-> turn (fixture/with-field "scope" "Session") (fixture/with-field "prefix" "session "))
        active (fixture/scenario "active")]
    (is (= "turn 58k: in 1.2k out 3.4k think 1.1k cache read 52k write 1.9k" (text-of turn active)))
    (is (= "session 3.4M: in 12k out 210k think 70k cache read 3.0M write 180k" (text-of session active)))
    (is (nil? (preview/pieces turn (assoc active :token-spend {}))))))

(deftest session-cost-shows-dollars-and-cents
  (let [cost (fixture/create "SessionCost")]
    (is (= "$18.40" (text-of cost (fixture/scenario "active"))))
    (is (nil? (preview/pieces cost (assoc (fixture/scenario "active") :cost 0))))))

(deftest a-session-notice-shows-the-time-left-the-way-the-runtime-pads-it
  (let [notice (fixture/create "SessionNotice")]
    (is (= "📌 ✗ cargo test (exit 101) (9m55s)" (text-of notice (first scenarios/all))))
    (is (= "📌 ✗ cargo test (exit 101)" (text-of (fixture/with-field notice "show_remaining" false) (first scenarios/all))))))

(deftest text-pieces-are-cut-to-max-chars-with-an-ellipsis-and-zero-means-unlimited
  (is (= "абв…" (width/cut "абвгде" 4)))
  (is (= "абвгде" (width/cut "абвгде" 6)))
  (is (= "абвгде" (width/cut "абвгде" 0)))
  (is (= "» rebase th…" (text-of (fixture/with-field (fixture/create "MyLastPrompt") "max_chars" 10) (first scenarios/all)))))

(deftest a-spacer-takes-no-columns
  (is (zero? (width/display-width (text-of (fixture/create "Spacer") (first scenarios/all))))))

(deftest git-state-has-the-runtime-space-before-the-state-marker
  (let [drawn (preview/git-branch {"color" {:kind :rgb :hex "#94e2d5"}
                                   "state_color" {:kind :rgb :hex "#fab387"}
                                   "show_worktree" false
                                   "show_state" true
                                   "show_ahead_behind" true}
                                  {:git true :worktree false :branch "main" :state "REBASE 2/5" :ahead 1 :behind 3})]
    (is (= "main [REBASE 2/5](↑1 ↓3)" (apply str (map :text drawn))))))

(deftest git-diff-separates-counts-with-the-separator-colour
  (let [diff (fixture/create "GitDiff")
        drawn (preview/pieces diff (fixture/scenario "pressure") {:kind :rgb :hex "#123456"})]
    (is (= ["~8" " " "+2" " " "-1"] (map :text drawn)))
    (is (= "#123456" (:colour (second drawn))))))
