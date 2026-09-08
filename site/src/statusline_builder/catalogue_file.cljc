(ns statusline-builder.catalogue-file
  (:require [statusline-builder.catalogue :as catalogue]
            #?(:clj [cheshire.core :as json])
            #?(:clj [clojure.java.io :as io])))

(def path "../site/segment-catalog.json")

(def ^:private missing "segment-catalog.json is missing: run `cargo run -- --schema > site/segment-catalog.json`\n")

(defn- exists? []
  #?(:cljs (.existsSync (js/require "fs") path)
     :clj (.exists (io/file path))))

(defn- parse [text]
  #?(:cljs (js->clj (js/JSON.parse text) :keywordize-keys true)
     :clj (json/parse-string text true)))

(defn- read-text []
  #?(:cljs (.readFileSync (js/require "fs") path "utf8")
     :clj (slurp path)))

(defn read-catalogue []
  (when-not (exists?)
    #?(:cljs (do (.write js/process.stderr missing) (js/process.exit 1))
       :clj (do (binding [*out* *err*] (print missing) (flush)) (System/exit 1))))
  (catalogue/validate (parse (read-text))))
