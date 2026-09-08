(ns statusline-builder.shell-words
  (:require [clojure.string :as str]))

(defn- whitespace? [character]
  (boolean (re-matches #"\s" (str character))))

(defn split [line]
  (loop [characters (seq line) word "" in-word? false quote nil words []]
    (if-let [character (first characters)]
      (let [rest-characters (next characters)
            escaped (second characters)]
        (cond
          quote
          (cond
            (= character quote) (recur rest-characters word in-word? nil words)
            (and (= character \\) (= quote \") escaped) (recur (next rest-characters) (str word escaped) in-word? quote words)
            :else (recur rest-characters (str word character) in-word? quote words))

          (or (= character \') (= character \"))
          (recur rest-characters word true character words)

          (and (= character \\) escaped)
          (recur (next rest-characters) (str word escaped) true nil words)

          (whitespace? character)
          (recur rest-characters "" false nil (if in-word? (conj words word) words))

          :else
          (recur rest-characters (str word character) true nil words)))
      (if in-word? (conj words word) words))))

(defn- plain? [word]
  (boolean (re-matches #"[A-Za-z0-9_@%+=:,./-]+" word)))

(defn join [words]
  (str/join " " (map (fn [word]
                       (if (plain? word)
                         word
                         (str "'" (str/replace word "'" "'\\''") "'")))
                     words)))
