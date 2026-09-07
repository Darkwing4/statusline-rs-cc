(ns statusline-builder.ron-test
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is]]
            [statusline-builder.fixture :as fixture]
            [statusline-builder.ron :as ron]))

(deftest every-piece-serialises-every-field-the-catalogue-declares
  (doseq [module fixture/modules]
    (let [lines (ron/segment->ron fixture/installed (fixture/create (:id module)))]
      (is (= (str "        " (:type module) "(") (first lines)))
      (doseq [field (:fields module)]
        (is (some #(str/starts-with? % (str "            " (:name field) ": ")) lines) (str (:id module) "." (:name field)))))))

(deftest lists-and-pairs-serialise-as-ron-arrays
  (let [model (fixture/with-field (fixture/create "Model") "replacements" [["Opus 5 (1M context)" "Opus"] ["Sonnet 5" "Sonnet"]])
        answer (fixture/with-field (fixture/create "LlmAnswer") "args" ["exec" "-s" "read-only"])]
    (is (some #{"            replacements: [(\"Opus 5 (1M context)\", \"Opus\"), (\"Sonnet 5\", \"Sonnet\")],"} (ron/segment->ron fixture/installed model)))
    (is (some #{"            args: [\"exec\", \"-s\", \"read-only\"],"} (ron/segment->ron fixture/installed answer)))))

(deftest a-pair-without-a-left-side-is-reported-instead-of-silently-dropped
  (let [state (fixture/fresh)
        model (some #(when (= "Model" (:type %)) %) (:segments state))
        broken (assoc state :segments (mapv #(if (= (:id model) (:id %)) (fixture/with-field % "replacements" [["" "Opus"]]) %) (:segments state)))]
    (is (= ["Model: every replacements row needs a left side."] (ron/config-problems broken)))
    (is (= [] (ron/config-problems state)))))

(deftest the-default-line-renders-the-default-config-shape
  (let [text (ron/generate (fixture/fresh))]
    (is (str/starts-with? text "(\n    separator: \" \",\n    separator_color: Rgb(120, 125, 140),\n    segments: [\n"))
    (is (str/includes? text "        Model(\n            color: Rgb(180, 142, 173),\n            prefix: \"\",\n            replacements: [],\n        ),"))
    (is (str/includes? text "            window: FiveHour,"))
    (is (str/includes? text "            window: SevenDay,"))
    (is (str/ends-with? (str/trimr text) "    ],\n)"))))

(deftest strings-and-floats-follow-the-ron-grammar
  (let [answer (-> (fixture/create "LlmAnswer") (fixture/with-field "prompt" "say \"hi\"\n\\done"))
        limit (-> (fixture/create "RateLimit:FiveHour") (fixture/with-field "gradient_midpoint_percentage" 42.5))]
    (is (some #{"            prompt: \"say \\\"hi\\\"\\n\\\\done\","} (ron/segment->ron fixture/installed answer)))
    (is (some #{"            gradient_midpoint_percentage: 42.5,"} (ron/segment->ron fixture/installed limit)))))
