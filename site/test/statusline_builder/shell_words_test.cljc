(ns statusline-builder.shell-words-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.shell-words :as shell-words]))

(deftest a-command-line-splits-like-a-shell-and-joins-back-unchanged
  (let [words ["codex" "exec" "-c" "approval_policy=never" "two words" "it's" ""]]
    (is (= words (shell-words/split (shell-words/join words)))))
  (is (= ["codex" "exec" "-s" "read-only"] (shell-words/split "  codex   exec\t-s read-only ")))
  (is (= ["say" "hello world" "a b" "esc aped"] (shell-words/split "say \"hello world\" 'a b' esc\\ aped")))
  (is (= ["open" "unterminated"] (shell-words/split "open 'unterminated")))
  (is (= [] (shell-words/split "")))
  (is (= "codex exec" (shell-words/join ["codex" "exec"])))
  (is (= "'a b' 'c'\\''d'" (shell-words/join ["a b" "c'd"]))))
