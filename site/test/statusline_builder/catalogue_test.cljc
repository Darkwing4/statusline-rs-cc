(ns statusline-builder.catalogue-test
  (:require [clojure.test :refer [deftest is]]
            [statusline-builder.catalogue :as catalogue]
            [statusline-builder.fixture :as fixture]
            [statusline-builder.segment :as segment]))

(deftest every-rate-limit-window-gets-its-own-shelf-piece
  (let [windows (->> (:segments fixture/catalogue)
                     (some #(when (= "RateLimit" (:name %)) %))
                     :fields
                     (some #(when (= "window" (:name %)) %))
                     :variants)]
    (doseq [window windows]
      (let [module (some #(when (= (str "RateLimit:" window) (:id %)) %) fixture/modules)]
        (is (some? module) window)
        (is (= window (get-in (segment/create module) [:config "window"])))))))

(deftest presets-only-name-fields-the-catalogue-declares
  (doseq [[type preset] segment/presets]
    (let [entry (some #(when (= type (:name %)) %) (:segments fixture/catalogue))]
      (is (some? entry) type)
      (doseq [key (keys preset)]
        (is (some #(= key (:name %)) (:fields entry)) (str type "." key))))))

(deftest segment-names-become-readable-labels
  (is (= "Prompt cache TTL" (catalogue/segment-label "PromptCacheTtl")))
  (is (= "LLM insight" (catalogue/segment-label "LlmInsight")))
  (is (= "My last prompt" (catalogue/segment-label "MyLastPrompt")))
  (is (= "Cwd" (catalogue/segment-label "Cwd"))))

(deftest field-names-keep-their-units-readable
  (is (= "Initial scan KiB" (catalogue/field-label "initial_scan_kib")))
  (is (= "TTL seconds" (catalogue/field-label "ttl_seconds")))
  (is (= "Max chars" (catalogue/field-label "max_chars"))))
