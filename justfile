package:
  cargo c
  makepkg -fi

clean:
  rm -rfv *.zst
  rm -rfv .build
  cargo clean -vv
