(ns statusline-builder.catalogue-file
  (:require [statusline-builder.catalogue :as catalogue]
            #?(:clj [cheshire.core :as json])))

(def path "../site/segment-catalog.json")

(defn- parse [text]
  #?(:cljs (js->clj (js/JSON.parse text) :keywordize-keys true)
     :clj (json/parse-string text true)))

(defn- read-text []
  #?(:cljs (.readFileSync (js/require "fs") path "utf8")
     :clj (slurp path)))

(defn read-catalogue []
  (catalogue/validate (parse (read-text))))
