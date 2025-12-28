(defsystem "kanjinet"
  :version "0.0.0"
  :author "Colin Woodbury <colin@fosskers.ca>"
  :license "MPL-2.0"
  :homepage "https://github.com/fosskers/kanji-net"
  :depends-on (:filepaths :simple-graph :transducers :parcom)
  :serial t
  :components ((:module "lisp"
                :components ((:file "package")))))
