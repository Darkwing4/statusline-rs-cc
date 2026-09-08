(ns statusline-builder.editor-state-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.editor-state :as es]
            [statusline-builder.fixture :as fixture]
            [statusline-builder.segment :as segment]))

(deftest every-catalogue-segment-is-on-the-shelf-with-its-pitch
  (let [shelf (es/shelf-modules (assoc (fixture/fresh) :segments []))]
    (doseq [entry (:segments fixture/catalogue)]
      (let [matching (filter #(= (:name entry) (:type %)) shelf)]
        (is (seq matching) (:name entry))
        (doseq [module matching]
          (is (= (:pitch entry) (:description module))))))))

(deftest the-default-line-only-uses-pieces-the-catalogue-knows
  (doseq [entry segment/default-line]
    (let [id (if (string? entry) entry (:module entry))
          module (some #(when (= id (:id %)) %) fixture/modules)]
      (is (some? module) id)
      (when-not (string? entry)
        (let [fields (set (map :name (:fields module)))]
          (doseq [name (keys (:config entry))]
            (is (contains? fields name) (str id "." name)))))))
  (let [insights (filter #(= "LlmInsight" (:type %)) (:segments (fixture/fresh)))]
    (is (= 2 (count insights)))
    (is (not= (get-in (first insights) [:config "prompt"]) (get-in (second insights) [:config "prompt"])))
    (is (every? #(get-in % [:config "standalone"]) insights))))

(deftest standalone-pieces-come-from-the-config-or-from-the-segment-itself
  (is (segment/standalone? (fixture/create "CommandOutput")))
  (is (segment/standalone? (fixture/create "SessionNotice")))
  (is (not (segment/standalone? (fixture/create "Reminder"))))
  (is (segment/standalone? (assoc-in (fixture/create "Spacer") [:config "shape"] "BlankLine")))
  (is (not (segment/standalone? (fixture/create "Spacer"))))
  (is (segment/line-break? (fixture/create "Spacer")))
  (is (not (segment/line-break? (assoc-in (fixture/create "Spacer") [:config "shape"] "Gap")))))

(deftest moving-a-piece-lands-it-before-the-requested-index
  (let [state (fixture/fresh)
        ids (mapv :id (:segments state))
        moved (es/move-segment-to state (ids 0) 3)]
    (is (= [(ids 1) (ids 2) (ids 0) (ids 3)] (subvec (mapv :id (:segments moved)) 0 4)))
    (is (= (ids 0) (:selected-id moved)))
    (is (identical? state (es/move-segment-to state (ids 2) 2)))
    (is (identical? state (es/add-segment state "Model" 0)))))

(deftest reordering-the-line-leaves-the-shelf-untouched
  (let [state (fixture/fresh)
        ids (mapv :id (:segments state))]
    (is (= (es/shelf-key state) (es/shelf-key (es/move-segment-to state (ids 0) 3))))
    (is (not= (es/shelf-key state) (es/shelf-key (es/remove-segment state (ids 0)))))))
