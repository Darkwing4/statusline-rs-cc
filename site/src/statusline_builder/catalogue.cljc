(ns statusline-builder.catalogue
  (:require [clojure.string :as str]))

(def ^:private window-labels {"FiveHour" "5h" "SevenDay" "7d"})

(def ^:private window-overrides
  {"FiveHour" {"prefix" "{t}h "}
   "Fable" {"prefix" "- {t}d " "active_marker" " |"}})

(def ^:private repeatable-segments #{"Spacer" "LlmInsight" "TokenSpend"})

(def ^:private unit-words {"ttl" "TTL" "llm" "LLM" "kib" "KiB" "cpu" "CPU"})

(defn segment-label [name]
  (->> (str/split (str/replace name #"([a-z0-9])([A-Z])" "$1 $2") #" ")
       (map-indexed (fn [index word] (or (unit-words (str/lower-case word)) (if (zero? index) word (str/lower-case word)))))
       (str/join " ")))

(defn field-label [name]
  (let [text (str/join " " (map #(get unit-words % %) (str/split name #"_")))]
    (str (str/upper-case (subs text 0 1)) (subs text 1))))

(defn- fail [message]
  (throw (ex-info message {})))

(defn validate [catalogue]
  (when-not (and (map? catalogue) (string? (:version catalogue)) (vector? (:segments catalogue)))
    (fail "The segment catalogue has an unexpected shape."))
  (when (empty? (:segments catalogue))
    (fail "The segment catalogue is empty."))
  (doseq [{:keys [name pitch fields]} (:segments catalogue)]
    (when-not (and (string? name) (string? pitch) (vector? fields))
      (fail "A catalogue entry is missing its name, pitch, or fields."))
    (doseq [field fields]
      (when-not (and (string? (:name field)) (string? (:kind field)) (string? (:hint field)))
        (fail (str name " has a field without a name, a kind, or a hint.")))
      (when (and (= "enum" (:kind field)) (empty? (:variants field)))
        (fail (str name "." (:name field) " is an enum without variants.")))))
  catalogue)

(defn- rate-limit-modules [segment base]
  (let [window (some #(when (and (= "window" (:name %)) (= "enum" (:kind %))) %) (:fields segment))]
    (when-not window
      (fail "RateLimit has no window enum to build the shelf from."))
    (for [variant (:variants window)]
      (assoc base
             :id (str "RateLimit:" variant)
             :label (str "Rate limit " (get window-labels variant variant))
             :overrides (merge {"window" variant "prefix" "{t}d "} (window-overrides variant))
             :repeatable false))))

(defn build-modules [catalogue]
  (vec (mapcat (fn [segment]
                 (let [base {:type (:name segment) :description (:pitch segment) :fields (:fields segment)}]
                   (if (= "RateLimit" (:name segment))
                     (rate-limit-modules segment base)
                     [(assoc base
                             :id (:name segment)
                             :label (segment-label (:name segment))
                             :overrides {}
                             :repeatable (contains? repeatable-segments (:name segment)))])))
               (:segments (validate catalogue)))))
