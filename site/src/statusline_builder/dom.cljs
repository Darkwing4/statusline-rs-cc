(ns statusline-builder.dom)

(defn $ [id]
  (.getElementById js/document id))

(defn el
  ([tag] (el tag "" ""))
  ([tag class] (el tag class ""))
  ([tag class text]
   (let [node (.createElement js/document tag)]
     (when (seq class)
       (set! (.-className node) class))
     (when (seq text)
       (set! (.-textContent node) text))
     node)))

(defn set-text! [id text]
  (set! (.-textContent ($ id)) text))

(defn set-hidden! [id hidden]
  (set! (.-hidden ($ id)) hidden))

(defn nodes [selector]
  (array-seq (.querySelectorAll js/document selector)))

(defn line-piece-nodes []
  (array-seq (.querySelectorAll ($ "lineCanvas") ".piece")))

(defn segment-id [node]
  (.getAttribute node "data-segment-id"))

(defn announce! [message]
  (set-text! "liveRegion" "")
  (js/requestAnimationFrame #(set-text! "liveRegion" message)))

(defn focus-piece! [id]
  (js/requestAnimationFrame
   (fn []
     (when-let [node (.querySelector ($ "lineCanvas") (str "[data-segment-id=\"" (js/CSS.escape id) "\"]"))]
       (.focus node)))))

(defn reduced-motion? []
  (.-matches (js/matchMedia "(prefers-reduced-motion: reduce)")))
