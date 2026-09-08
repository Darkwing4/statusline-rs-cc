(ns statusline-builder.editor-state
  (:require [statusline-builder.catalogue :as catalogue]
            [statusline-builder.colour :as colour]
            [statusline-builder.scenarios :as scenarios]
            [statusline-builder.segment :as segment]))

(def line-selection :line)

(def initial
  {:modules []
   :catalogue-version ""
   :separator " "
   :separator-color (colour/grey)
   :terminal-width 120
   :scenario-id "active"
   :segments []
   :selected-id line-selection})

(defonce state (atom initial))

(defn module-entry [state module-id]
  (or (some #(when (= module-id (:id %)) %) (:modules state))
      (throw (ex-info (str "Unknown module: " module-id) {}))))

(defn segment-module [state segment]
  (module-entry state (:module-id segment)))

(defn segment [state id]
  (some #(when (= id (:id %)) %) (:segments state)))

(defn segment-index [state id]
  (first (keep-indexed (fn [index candidate] (when (= id (:id candidate)) index)) (:segments state))))

(defn selected-segment [state]
  (segment state (:selected-id state)))

(defn used-module-ids [state]
  (set (map :module-id (:segments state))))

(defn shelf-modules [state]
  (let [used (used-module-ids state)]
    (filterv #(or (:repeatable %) (not (contains? used (:id %)))) (:modules state))))

(defn scenario [state]
  (or (some #(when (= (:scenario-id state) (:id %)) %) scenarios/all)
      (first scenarios/all)))

(defn install-catalogue [state catalogue]
  (assoc state
         :modules (catalogue/build-modules catalogue)
         :catalogue-version (:version catalogue)))

(defn- default-segments [state]
  (mapv (fn [entry]
          (if (string? entry)
            (segment/create (module-entry state entry))
            (update (segment/create (module-entry state (:module entry))) :config merge (:config entry))))
        segment/default-line))

(defn reset [state]
  (merge state
         (dissoc initial :modules :catalogue-version)
         {:segments (default-segments state)}))

(defn- insert-at [items index item]
  (vec (concat (subvec items 0 index) [item] (subvec items index))))

(defn- remove-at [items index]
  (vec (concat (subvec items 0 index) (subvec items (inc index)))))

(defn add-segment [state module-id index]
  (let [module (module-entry state module-id)]
    (if (and (not (:repeatable module)) (contains? (used-module-ids state) module-id))
      state
      (let [created (segment/create module)
            target (max 0 (min index (count (:segments state))))]
        (-> state
            (update :segments insert-at target created)
            (assoc :selected-id (:id created)))))))

(defn remove-segment [state id]
  (if-let [index (segment-index state id)]
    (cond-> (update state :segments remove-at index)
      (= id (:selected-id state)) (assoc :selected-id line-selection))
    state))

(defn move-segment [state id delta]
  (let [from (segment-index state id)
        to (when from (+ from delta))]
    (if (and from (<= 0 to) (< to (count (:segments state))))
      (let [moved (get-in state [:segments from])]
        (update state :segments #(insert-at (remove-at % from) to moved)))
      state)))

(defn move-segment-to [state id requested-index]
  (if-let [from (segment-index state id)]
    (let [segments (:segments state)
          bounded (max 0 (min requested-index (count segments)))
          to (if (< from bounded) (dec bounded) bounded)]
      (if (= from to)
        state
        (assoc state
               :segments (insert-at (remove-at segments from) to (segments from))
               :selected-id id)))
    state))

(defn set-field [state id field value]
  (update state :segments (fn [segments]
                            (mapv #(if (= id (:id %)) (assoc-in % [:config field] value) %) segments))))

(defn select [state target]
  (assoc state :selected-id target))

(defn config-key [state]
  (select-keys state [:separator :separator-color :segments]))

(defn line-key [state]
  (select-keys state [:separator :separator-color :segments :terminal-width :scenario-id]))

(defn shelf-key [state]
  (used-module-ids state))

(defn inspector-key [state]
  (if-let [selected (selected-segment state)]
    [(:id selected)
     (mapv (fn [field]
             (let [value (get-in selected [:config (:name field)])]
               (case (:kind field)
                 "color" (:kind value)
                 ("list" "pairs") (count value)
                 nil)))
           (:fields (segment-module state selected)))]
    [line-selection (:kind (:separator-color state))]))
