(ns statusline-builder.fixture
  (:require [statusline-builder.catalogue-file :as catalogue-file]
            [statusline-builder.editor-state :as es]
            [statusline-builder.scenarios :as scenarios]
            [statusline-builder.segment :as segment]))

(def catalogue (catalogue-file/read-catalogue))

(def installed (es/install-catalogue es/initial catalogue))

(def modules (:modules installed))

(defn fresh []
  (es/reset installed))

(defn create [module-id]
  (segment/create (es/module-entry installed module-id)))

(defn scenario [id]
  (some #(when (= id (:id %)) %) scenarios/all))

(defn with-field [segment field value]
  (assoc-in segment [:config field] value))
